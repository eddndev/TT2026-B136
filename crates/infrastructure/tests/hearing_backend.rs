mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;

use application::hearings::*;
use domain::identity::Role;
use hearing_database_support::*;

#[test]
fn exact_hearing_history_preserves_receipts_across_schedule_replacement_and_cancellation() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let service = service(&db, db.owner, Role::Owner);
    let command = schedule();
    let before = counts(&mut db);
    let prepared = service
        .prepare("session", db.case, command.clone())
        .unwrap();
    assert_eq!(
        counts(&mut db),
        before,
        "preparation must not reserve an operation or hearing"
    );
    let first = service
        .submit(
            "session",
            db.case,
            command.clone(),
            prepared.submission_digest,
        )
        .unwrap();
    assert_eq!(first.snapshot.revision.get(), 1);
    assert_eq!(first.snapshot.receipt.operation_id, command.operation_id);
    assert_eq!(
        first.snapshot.receipt.submission_digest,
        prepared.submission_digest
    );
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 1, before.2 + 1));
    let second = persist(
        &service,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: command.hearing_id,
            change: HearingChange::Replace {
                expected_revision: first.snapshot.revision,
                context: context(),
                values: values("2026-09-16T10:00:00-06:00"),
                reason: HearingNote::new("New communicated date").unwrap(),
            },
        },
    );
    let third = persist(
        &service,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: command.hearing_id,
            change: HearingChange::Cancel {
                expected_revision: second.snapshot.revision,
                reason: HearingNote::new("Appointment cancelled").unwrap(),
            },
        },
    );
    assert_eq!(third.snapshot.status, HearingStatus::Cancelled);
    assert_eq!(third.snapshot.values, second.snapshot.values);
    assert_eq!(
        third.snapshot.scheduling_context,
        second.snapshot.scheduling_context
    );
    assert_eq!(
        service
            .get(
                "session",
                db.case,
                command.hearing_id,
                Some(first.snapshot.revision)
            )
            .unwrap(),
        first
    );
    let history = service
        .history(
            "session",
            db.case,
            command.hearing_id,
            HearingHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(history.revisions, vec![third, second, first]);
    assert!(!history.has_more);
}
