mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;

use application::{documents::StageSupportReadLimits, hearing_results::*};
use domain::identity::Role;
use hearing_result_database_support::*;

#[test]
fn result_preparation_loads_an_exact_programming_source_without_reserving_state() {
    let Some(mut db) = Fixture::new() else { return };
    hearing_database_support::complete(&mut db);
    let hearing_service = hearing_database_support::service(&db, db.owner, Role::Owner);
    let anchor = hearing_database_support::persist(
        &hearing_service,
        db.case,
        hearing_database_support::schedule(),
    );
    let before = counts(&mut db);
    let prepared = store(&db)
        .prepare(
            db.owner,
            db.case,
            &record(anchor.snapshot.id),
            &StageSupportReadLimits::standard(),
        )
        .unwrap();
    assert_eq!(prepared.anchor, anchor);
    assert_eq!(prepared.case_id, db.case);
    assert!(prepared.base.is_none());
    assert!(prepared.continuation.is_none());
    assert_eq!(counts(&mut db), before);
}

#[test]
fn exact_result_history_preserves_receipts_across_record_correction_and_withdrawal() {
    let Some(mut db) = Fixture::new() else { return };
    hearing_database_support::complete(&mut db);
    let hearing_service = hearing_database_support::service(&db, db.owner, Role::Owner);
    let anchor = hearing_database_support::persist(
        &hearing_service,
        db.case,
        hearing_database_support::schedule(),
    );
    let service = service(&db, db.owner, Role::Owner);
    let command = record(anchor.snapshot.id);
    let before = counts(&mut db);
    let prepared = service
        .prepare("session", db.case, command.clone())
        .unwrap();
    assert_eq!(counts(&mut db), before);
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
    let second = persist(
        &service,
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: command.hearing_id,
            result_id: command.result_id,
            change: HearingResultChange::Correct {
                expected_revision: first.snapshot.revision,
                values: values("Corrected declaration"),
                reason: HearingResultText::new("Transcription correction").unwrap(),
            },
        },
    );
    let third = persist(
        &service,
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: command.hearing_id,
            result_id: command.result_id,
            change: HearingResultChange::Withdraw {
                expected_revision: second.snapshot.revision,
                reason: HearingResultText::new("Withdraw incorrect registration").unwrap(),
            },
        },
    );
    assert_eq!(third.snapshot.status, HearingResultStatus::Withdrawn);
    assert_eq!(third.snapshot.values, second.snapshot.values);
    assert_eq!(third.snapshot.anchor, first.snapshot.anchor);
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 3, before.2 + 3));
    assert_eq!(
        service
            .get(
                "session",
                db.case,
                command.hearing_id,
                command.result_id,
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
            command.result_id,
            HearingResultHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        history.revisions,
        vec![
            HearingResultHistoryEntry::from(&third.snapshot),
            HearingResultHistoryEntry::from(&second.snapshot),
            HearingResultHistoryEntry::from(&first.snapshot)
        ]
    );
    assert!(!history.has_more);
}
