use super::*;
use crate::measure_fixture::{FixedClock, TestIdentity};
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use std::sync::Arc;

mod fixture;
mod integrity;
use fixture::*;

fn reads(db: &Fixture, actor: Principal) -> PrecautionaryHearingRecordReadService {
    PrecautionaryHearingRecordReadService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

#[test]
fn mixed_hearing_read_selectors_preserve_cancelled_head_and_exact_older_c_and_m2_targets() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, captures, other) = chain(&mut db);
    let reader = reads(&db, seed.actor);
    let first = &captures[0];
    let last = &captures[2];
    let id = first.capture.review.command.hearing_id;
    same_operation(&reader.get("session", db.case, id, None).unwrap(), last);
    for original in &captures {
        same_operation(
            &reader
                .get(
                    "session",
                    db.case,
                    id,
                    Some(original.capture.review.result_revision),
                )
                .unwrap(),
            original,
        );
        same_operation(
            &reader
                .get_operation(
                    "session",
                    db.case,
                    original.capture.review.command.operation_id,
                )
                .unwrap(),
            original,
        );
    }
    assert_eq!(
        last.capture.review.status,
        domain::hearings::HearingStatus::Cancelled
    );
    assert!(first.history.record_history.decisions.is_empty());
    assert_eq!(first.history.record_history.records.administrative.len(), 1);
    assert_eq!(last.history.record_history.decisions.len(), 1);
    assert_eq!(last.history.captures.len(), 3);
    let mut expected = [last, &other];
    expected.sort_by_key(|h| h.capture.review.command.hearing_id.as_uuid());
    let page = reader
        .list(
            "session",
            db.case,
            PrecautionaryHearingReadQuery::new(1, None).unwrap(),
        )
        .unwrap();
    assert_eq!(page.case_id, db.case);
    assert_eq!(page.items.len(), 1);
    same_operation(&page.items[0], expected[0]);
    assert!(page.has_more);
    assert_eq!(
        page.next_after_id,
        Some(expected[0].capture.review.command.hearing_id)
    );
    let tail = reader
        .list(
            "session",
            db.case,
            PrecautionaryHearingReadQuery::new(1, page.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(tail.items.len(), 1);
    same_operation(&tail.items[0], expected[1]);
    assert!(!tail.has_more);
    assert_eq!(tail.next_after_id, None);
    let actions: Vec<String> = db.admin.query("SELECT action FROM audit_events WHERE action IN
        ('precautionary_hearing.read','precautionary_hearing.operation','precautionary_hearing.list') ORDER BY sequence", &[])
        .unwrap().into_iter().map(|r| r.get(0)).collect();
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "precautionary_hearing.read")
            .count(),
        4
    );
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "precautionary_hearing.operation")
            .count(),
        3
    );
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "precautionary_hearing.list")
            .count(),
        2
    );
}

#[test]
fn mixed_hearing_reads_require_current_full_principal_and_case_even_for_closed_history() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command);
    let original_case = db.case;
    let user = db.user("paralegal", true);
    let actor = crate::measure_fixture::principal(&mut db, user);
    let storage = store(&db);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(
                seed.corrected
                    .capture
                    .review
                    .command
                    .context
                    .administration_revision,
            ),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let id = original.capture.review.command.hearing_id;
    same_operation(
        &reads(&db, actor.clone())
            .get("session", db.case, id, None)
            .unwrap(),
        &original,
    );
    let stale = Principal {
        email: "stale-hearing-reader@example.test".into(),
        ..actor.clone()
    };
    denied(&mut db, &storage, &stale, &original);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &user.as_uuid()],
        )
        .unwrap();
    denied(&mut db, &storage, &actor, &original);
    let _other = crate::measure_fixture::setup(&mut db);
    let reader = reads(&db, seed.actor);
    let before = snapshot(&mut db);
    assert!(reader.get("session", db.case, id, None).is_err());
    assert!(reader
        .get_operation(
            "session",
            db.case,
            original.capture.review.command.operation_id
        )
        .is_err());
    assert!(reader
        .get(
            "session",
            original_case,
            PrecautionaryHearingId::new(),
            None
        )
        .is_err());
    assert!(reader
        .get(
            "session",
            original_case,
            id,
            Some(PrecautionaryHearingRevision::new(2).unwrap())
        )
        .is_err());
    assert!(reader
        .get_operation(
            "session",
            original_case,
            PrecautionaryHearingOperationId::new()
        )
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    let empty = reader
        .list("session", db.case, PrecautionaryHearingReadQuery::default())
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_id, None);
}

fn denied(
    db: &mut Fixture,
    storage: &PostgresPrecautionaryHearingStore,
    actor: &Principal,
    original: &PrecautionaryHearingRecordStoredOperation,
) {
    let before = snapshot(db);
    let command = &original.capture.review.command;
    assert!(PrecautionaryHearingRecordReadStore::get(
        storage,
        actor,
        db.case,
        command.hearing_id,
        None
    )
    .is_err());
    assert!(PrecautionaryHearingRecordReadStore::get_operation(
        storage,
        actor,
        db.case,
        command.operation_id
    )
    .is_err());
    assert!(PrecautionaryHearingRecordReadStore::list(
        storage,
        actor,
        db.case,
        PrecautionaryHearingReadQuery::default()
    )
    .is_err());
    assert_eq!(snapshot(db), before);
}

fn snapshot(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({"measures": crate::administrative_fixture::snapshot(db),
        "hearings": crate::hearing_fixture::snapshot(db)})
}
