mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_storage_support;
mod procedural_fact_backend_support;

use application::{deadlines::*, ApplicationError};
use deadline_backend_support::{persist_legacy, service, setup, Fixture};
use deadline_storage_support::*;
use domain::{identity::Role, procedural_time::DeclaredProceduralTime};
use serde_json::json;

#[test]
fn attention_preserves_declared_precision_offset_and_historical_calculation() {
    let Some(db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let original = persist_legacy(&db, db.owner, command);
    let date = "2026-01-09".parse().unwrap();
    let times = [
        DeclaredProceduralTime::unknown(),
        DeclaredProceduralTime::date(date, None).unwrap(),
        DeclaredProceduralTime::date(date, Some(time::UtcOffset::UTC)).unwrap(),
        DeclaredProceduralTime::minute(
            date,
            10,
            12,
            Some(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
        )
        .unwrap(),
        DeclaredProceduralTime::second(
            date,
            10,
            12,
            0,
            Some(time::UtcOffset::from_hms(14, 0, 0).unwrap()),
        )
        .unwrap(),
    ];
    let mut base = original.clone();
    for at in times {
        base = persist_legacy(&db, db.owner, attention(&base, at));
        let exact = workflow
            .get("session", db.case, base.id, Some(base.revision))
            .unwrap();
        assert_eq!(exact.attention, base.attention);
        let DeadlineAttention::Recorded { occurred_at, .. } = exact.attention else {
            panic!("recorded attention expected")
        };
        assert_eq!(occurred_at, at);
        assert_eq!(exact.calculation, original.calculation);
        assert_eq!(exact.definition, original.definition);
    }
    let retired = persist_legacy(&db, db.owner, retire(&base));
    assert_eq!(retired.attention, base.attention);
    assert_eq!(retired.calculation, original.calculation);
    assert_eq!(
        workflow
            .get("session", db.case, original.id, Some(original.revision))
            .unwrap(),
        original
    );
}

#[test]
fn attention_json_rejects_ignored_keys_missing_offset_and_silent_normalization() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let initial = persist_legacy(&db, db.owner, command);
    let at = DeclaredProceduralTime::date("2026-01-09".parse().unwrap(), None).unwrap();
    let base = persist_legacy(&db, db.owner, attention(&initial, at));
    let valid = json!({"status":"recorded","occurred_at":{"precision":"date","year":2026,"month":1,"day":9,"offset_seconds":null},"statement":"Declared action","locator":"Captured record"});
    let mut extra = valid.clone();
    extra["ignored"] = json!(true);
    let mut missing_offset = valid.clone();
    missing_offset["occurred_at"]
        .as_object_mut()
        .unwrap()
        .remove("offset_seconds");
    let mut invented_hour = valid.clone();
    invented_hour["occurred_at"]["hour"] = json!(0);
    let mut whitespace = valid.clone();
    whitespace["statement"] = json!("Declared action ");
    remove_checks(&mut db);
    let before = audit(&mut db);
    for invalid in [extra, missing_offset, invented_hour, whitespace] {
        set_attention(&mut db, &base, &invalid);
        assert!(matches!(
            workflow.get("session", db.case, base.id, Some(base.revision)),
            Err(ApplicationError::Deadline(
                DeadlineError::StoredInconsistent(_)
            ))
        ));
    }
    assert_eq!(audit(&mut db), before);
}

#[test]
fn strict_binary_inputs_and_results_reject_trailing_bytes_before_returning_history() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let base = persist_legacy(&db, db.owner, command);
    let row = db.admin.query_one("SELECT input_canonical,result_canonical FROM case_deadline_revisions WHERE deadline_id=$1 AND revision=1", &[&base.id.as_uuid()]).unwrap();
    let input: Vec<u8> = row.get(0);
    let result: Vec<u8> = row.get(1);
    remove_checks(&mut db);
    // A generated projection also rejects malformed input during the write. Remove
    // it only in this privileged damage fixture to exercise the reader's decoder.
    db.admin
        .batch_execute(
            "ALTER TABLE case_deadline_revisions ALTER COLUMN input_view DROP EXPRESSION",
        )
        .unwrap();
    let before = audit(&mut db);
    for column in ["input_canonical", "result_canonical"] {
        db.admin.execute("UPDATE case_deadline_revisions SET input_canonical=$2,result_canonical=$3 WHERE deadline_id=$1", &[&base.id.as_uuid(), &input, &result]).unwrap();
        let mut changed = if column == "input_canonical" {
            input.clone()
        } else {
            result.clone()
        };
        changed.push(0);
        db.admin
            .execute(
                &format!("UPDATE case_deadline_revisions SET {column}=$2 WHERE deadline_id=$1"),
                &[&base.id.as_uuid(), &changed],
            )
            .unwrap();
        assert!(workflow
            .get("session", db.case, base.id, Some(base.revision))
            .is_err());
    }
    assert_eq!(audit(&mut db), before);
}

#[test]
fn stored_canonical_review_cannot_disagree_with_the_captured_fields() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let base = persist_legacy(&db, db.owner, command);
    remove_checks(&mut db);
    db.admin.execute("UPDATE case_deadline_revisions SET review_canonical=review_canonical || '\\x00'::bytea WHERE deadline_id=$1", &[&base.id.as_uuid()]).unwrap();
    let before = audit(&mut db);
    assert!(workflow.get("session", db.case, base.id, None).is_err());
    assert_eq!(audit(&mut db), before);
}

#[test]
fn valid_receipt_does_not_allow_attention_to_replace_a_previous_definition() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist_legacy(&db, db.owner, command);
    let corrected = persist_legacy(&db, db.owner, change_title(&first));
    remove_checks(&mut db);
    change_action(&mut db, &corrected, DeadlineAction::SetAttention);
    let before = audit(&mut db);
    assert!(workflow
        .get("session", db.case, first.id, Some(corrected.revision))
        .is_err());
    assert_eq!(audit(&mut db), before);
}

#[test]
fn valid_receipt_does_not_allow_retirement_to_replace_a_previous_definition() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist_legacy(&db, db.owner, command);
    let corrected = persist_legacy(&db, db.owner, change_title(&first));
    remove_checks(&mut db);
    change_action(&mut db, &corrected, DeadlineAction::Retire);
    let before = audit(&mut db);
    assert!(workflow
        .get("session", db.case, first.id, Some(corrected.revision))
        .is_err());
    assert_eq!(audit(&mut db), before);
}
