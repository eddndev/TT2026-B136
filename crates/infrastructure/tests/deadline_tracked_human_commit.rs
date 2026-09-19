mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_guard_support;
mod procedural_fact_backend_support;

use application::deadlines::*;
use deadline_backend_support::*;
use deadline_tracked_backend_support::*;
use deadline_tracked_guard_support::*;
use domain::identity::Role;
use infrastructure::RingSha256Hasher;

#[test]
fn tracked_human_successors_retain_the_real_author_responsible_and_exact_audit_targets() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("litigator", true);
    let email = format!("{actor}@example.test");
    let responsible = db.user("paralegal", true);
    let profile = profile(&db);
    let source = source(&db);
    let repository = store(&db);
    let mut command = command(&db, &profile, &source);
    definition_mut(&mut command).responsible = responsible;
    let prepared = prepare_as(
        &db,
        repository.as_ref(),
        &command,
        actor,
        &email,
        Some(fixed_profile()),
    );
    let first = repository.commit(actor, prepared).unwrap();
    let first_row = revision_row(&mut db, &first);
    advance_profile(&db, &profile);
    let mut revisions = vec![first.clone()];
    for action in 0..3 {
        let prior = revisions.last().unwrap();
        let command = match action {
            0 => correct(prior),
            1 => attention(prior),
            _ => retire(prior),
        };
        let policies = if action == 0 {
            Some(fixed_profile())
        } else {
            None
        };
        let prepared = prepare_as(&db, repository.as_ref(), &command, actor, &email, policies);
        let next = repository.commit(actor, prepared).unwrap();
        deadline_successor_matches(&RingSha256Hasher, prior, &next).unwrap();
        assert_eq!(next.recorded_by.user_id(), Some(actor));
        assert_eq!(next.recorded_by.email(), Some(email.as_str()));
        assert_eq!(next.responsible.id, responsible);
        assert_eq!(next.responsible.role, Role::Paralegal);
        assert_eq!(next.definition.profile, first.definition.profile);
        revisions.push(next);
    }
    assert_eq!(revision_row(&mut db, &first), first_row);
    let actions = [
        "deadline.registered",
        "deadline.corrected",
        "deadline.attention_recorded",
        "deadline.retired",
    ];
    let rows = db
        .admin
        .query(
            "SELECT actor,action,resource FROM audit_events WHERE action=ANY($1) ORDER BY sequence",
            &[&&actions[..]],
        )
        .unwrap();
    assert_eq!(rows.len(), revisions.len());
    for ((row, detail), action) in rows.iter().zip(&revisions).zip(actions) {
        assert_eq!(row.get::<_, String>(0), email);
        assert_eq!(row.get::<_, String>(1), action);
        assert_eq!(
            row.get::<_, String>(2),
            format!(
                "case:{}:deadline:{}:revision:{}:operation:{}:submission:{}:capture:{}",
                db.case,
                detail.id,
                detail.revision.get(),
                detail.receipt.operation_id,
                detail.receipt.submission_digest.to_hex(),
                detail.receipt.capture_digest.to_hex()
            )
        );
        assert_eq!(detail.recorded_by.user_id(), Some(actor));
        assert_eq!(stored_evidence(&mut db, detail), canonical_evidence(detail));
    }
    assert_readback(&db, repository.as_ref(), &revisions);
}

#[test]
fn changed_or_fabricated_captured_email_cannot_rebind_a_confirmed_human_receipt() {
    for change_email_after_prepare in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let repository = store(&db);
        let email = if change_email_after_prepare {
            "owner@example.test"
        } else {
            "fabricated@example.test"
        };
        let prepared = prepare_as(
            &db,
            repository.as_ref(),
            &setup(&db),
            db.owner,
            email,
            Some(fixed_profile()),
        );
        deadline_receipt_matches(&RingSha256Hasher, &prepared_detail(&db, &prepared)).unwrap();
        if change_email_after_prepare {
            db.admin
                .execute(
                    "UPDATE users SET email='changed@example.test' WHERE id=$1",
                    &[&db.owner.as_uuid()],
                )
                .unwrap();
        }
        let before = snapshot(&mut db);
        assert!(repository.commit(db.owner, prepared).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn failed_audit_rolls_back_tracked_registration_and_human_successor() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &setup(&db), Some(fixed_profile()));
    let first = repository.commit(db.owner, prepared).unwrap();
    let initial = revision_row(&mut db, &first);
    let register = DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: DeadlineId::new(),
        change: DeadlineChange::Register {
            definition: first.definition.clone(),
        },
    };
    let changes = [
        tracked_prepared(&db, repository.as_ref(), &register, Some(fixed_profile())),
        tracked_prepared(
            &db,
            repository.as_ref(),
            &correct(&first),
            Some(fixed_profile()),
        ),
    ];
    db.admin
        .batch_execute(
            "CREATE FUNCTION reject_tracked_deadline_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected tracked audit failure'; END; $$;
        CREATE TRIGGER reject_tracked_deadline_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_tracked_deadline_audit()",
        )
        .unwrap();
    let before = snapshot(&mut db);
    for prepared in changes {
        assert!(repository.commit(db.owner, prepared).is_err());
        assert_eq!(snapshot(&mut db), before);
        assert_eq!(revision_row(&mut db, &first), initial);
    }
}
