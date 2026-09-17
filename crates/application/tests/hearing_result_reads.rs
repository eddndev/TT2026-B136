#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_result_support;
#[allow(dead_code)]
mod hearing_support;

use application::{hearing_results::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use hearing_result_support::*;
use std::sync::Arc;
#[test]
fn authorized_reads_forward_scope_cursor_exact_revision_and_audit_time() {
    let (identity, actor) = identity(Role::Paralegal, 3);
    let case_id = CaseId::new();
    let prep = preparation(case_id, actor.id);
    let cmd = command(&prep);
    let detail = detail(actor.id, &cmd, &prep);
    let expected = detail.clone();
    let query = HearingResultQuery::new(
        7,
        Some(HearingResultId::new()),
        HearingResultStatusFilter::All,
    )
    .unwrap();
    let history = HearingResultHistoryQuery::new(3, Some(8)).unwrap();
    let hearing_id = cmd.hearing_id;
    let id = cmd.result_id;
    let entry = HearingResultHistoryEntry::from(&detail.snapshot);
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |a, c, h, q, at| {
            assert_eq!(
                (a, c, h, q, at),
                (actor.id, case_id, hearing_id, query, instant())
            );
            Ok(HearingResultPage {
                results: vec![],
                has_more: false,
                next_after_id: None,
            })
        });
    store
        .expect_get()
        .times(1)
        .return_once(move |a, c, h, i, r, at| {
            assert_eq!(
                (a, c, h, i, r, at),
                (
                    actor.id,
                    case_id,
                    hearing_id,
                    id,
                    Some(HearingResultRevision::initial()),
                    instant()
                )
            );
            Ok(detail)
        });
    let returned = entry.clone();
    store
        .expect_history()
        .times(1)
        .return_once(move |a, c, h, i, q, at| {
            assert_eq!(
                (a, c, h, i, q, at),
                (actor.id, case_id, hearing_id, id, history, instant())
            );
            Ok(HearingResultHistoryPage {
                revisions: vec![returned],
                has_more: false,
                next_before_revision: None,
            })
        });
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    assert!(service
        .list("session", case_id, hearing_id, query)
        .unwrap()
        .results
        .is_empty());
    assert_eq!(
        service
            .get(
                "session",
                case_id,
                hearing_id,
                id,
                Some(HearingResultRevision::initial())
            )
            .unwrap(),
        expected
    );
    assert_eq!(
        service
            .history("session", case_id, hearing_id, id, history)
            .unwrap()
            .revisions,
        vec![entry]
    );
    assert_eq!(clock.calls(), 3);
}
#[test]
fn mismatched_scope_exact_revision_or_receipt_from_storage_is_not_disclosed() {
    for mode in 0..5 {
        let (identity, actor) = identity(Role::Paralegal, 1);
        let case_id = CaseId::new();
        let prep = preparation(case_id, actor.id);
        let cmd = command(&prep);
        let mut detail = detail(actor.id, &cmd, &prep);
        match mode {
            0 => detail.snapshot.case_id = CaseId::new(),
            1 => detail.snapshot.hearing_id = domain::hearings::HearingId::new(),
            2 => detail.snapshot.id = HearingResultId::new(),
            3 => detail.snapshot.revision = HearingResultRevision::new(2).unwrap(),
            _ => detail.snapshot.receipt.expected_revision = 2,
        }
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _, _| Ok(detail));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            matches!(
                service.get(
                    "session",
                    case_id,
                    cmd.hearing_id,
                    cmd.result_id,
                    Some(HearingResultRevision::initial())
                ),
                Err(ApplicationError::HearingResult(
                    HearingResultError::StoredInconsistent(_)
                ))
            ),
            "mode {mode}"
        );
    }
}
#[test]
fn lightweight_history_rejects_foreign_or_modified_receipts() {
    for mode in 0..4 {
        let (identity, actor) = identity(Role::Paralegal, 1);
        let case_id = CaseId::new();
        let prep = preparation(case_id, actor.id);
        let cmd = command(&prep);
        let mut entry = HearingResultHistoryEntry::from(&detail(actor.id, &cmd, &prep).snapshot);
        match mode {
            0 => entry.case_id = CaseId::new(),
            1 => entry.hearing_id = domain::hearings::HearingId::new(),
            2 => entry.id = HearingResultId::new(),
            _ => entry.receipt.operation_id = HearingResultOperationId::new(),
        }
        let mut store = MockStore::new();
        store
            .expect_history()
            .times(1)
            .return_once(move |_, _, _, _, _, _| {
                Ok(HearingResultHistoryPage {
                    revisions: vec![entry],
                    has_more: false,
                    next_before_revision: None,
                })
            });
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service
                .history(
                    "session",
                    case_id,
                    cmd.hearing_id,
                    cmd.result_id,
                    HearingResultHistoryQuery::new(10, None).unwrap()
                )
                .is_err(),
            "mode {mode}"
        );
    }
}
