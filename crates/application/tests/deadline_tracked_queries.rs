#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_service_support;
mod deadline_support;
mod deadline_technical_support;
mod deadline_tracked_support;
use application::{
    deadline_currentness::evaluate_deadline_currentness,
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::*,
};
use deadline_service_support::*;
use deadline_tracked_support::{accepted, pending, resign};
use domain::{crypto::Sha256Digest, identity::Role};

fn read_summary(
    row: DeadlineOverview,
    valid: bool,
) -> Result<DeadlinePage, application::ApplicationError> {
    let mut store = MockStore::new();
    store.expect_list().times(1).return_once(move |_, _, _, _| {
        Ok(DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        })
    });
    let (workflow, _) = service(store, identity(Role::Owner, if valid { 2 } else { 1 }));
    workflow.list("session", case_id(), list_query(20))
}
fn read_history(
    first: &DeadlineDetail,
    second: &DeadlineDetail,
    valid: bool,
) -> Result<DeadlineHistoryPage, application::ApplicationError> {
    let rows = vec![history_entry(second), history_entry(first)];
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(DeadlineHistoryPage {
                revisions: rows,
                has_more: false,
                next_before_revision: None,
            })
        });
    let (workflow, _) = service(store, identity(Role::Owner, if valid { 2 } else { 1 }));
    workflow.history("session", case_id(), first.id, history_query(20))
}
#[test]
fn only_current_accepted_active_summaries_may_publish_an_operational_due() {
    let base = deadline_technical_support::accepted(TrackingPolicy::Follow);
    let current = evaluate_deadline_currentness(
        hasher().as_ref(),
        &base,
        Some(&deadline_technical_support::heads(&base)),
        case_support::instant(),
    )
    .unwrap();
    let row = DeadlineOverview::from(&current);
    assert!(row.operational.due_at().is_some());
    read_summary(row.clone(), true).unwrap();
    for state in [
        DeadlineReviewState::Pending,
        DeadlineReviewState::LegacyUndeclared,
    ] {
        let mut changed = row.clone();
        changed.review_state = state;
        assert!(read_summary(changed, false).is_err());
    }
    let mut retired = row;
    retired.status = DeadlineStatus::Retired;
    assert!(read_summary(retired, false).is_err());
    let changed = deadline_technical_support::source_heads(&base, 2, false);
    let reviewed = deadline_technical_support::revision(
        &base,
        deadline_technical_support::event_command(deadline_technical_support::source_event(
            &changed, 2,
        )),
        changed,
    );
    for detail in [
        reviewed,
        deadline_technical_support::attention(&deadline_technical_support::legacy()),
        deadline_technical_support::retired(&base),
    ] {
        let row = historical_overview(&detail);
        assert!(row.calculation_due_at.is_some());
        assert_eq!(row.operational.due_at(), None);
        read_summary(row, true).unwrap();
    }
}
#[test]
fn a_valid_tracked_history_can_cross_human_and_technical_authors() {
    let first = accepted();
    let second = pending(&first);
    read_history(&first, &second, true).unwrap();
}
#[test]
fn history_checks_both_predecessor_commitments_after_individual_receipt_validation() {
    let first = accepted();
    for capture in [false, true] {
        let mut second = pending(&first);
        let DeadlineReceiptVersion::Tracked(metadata) = &mut second.receipt.version else {
            panic!("tracked receipt expected");
        };
        let prior = metadata.predecessor.as_mut().unwrap();
        if capture {
            prior.capture_digest = Sha256Digest::from_array([0x55; 32]);
        } else {
            prior.submission_digest = Sha256Digest::from_array([0x66; 32]);
        }
        resign(&mut second);
        assert!(read_history(&first, &second, false).is_err());
    }
}
#[test]
fn history_rejects_a_return_to_legacy_receipts_after_tracking_was_declared() {
    let first = accepted();
    let mut second = pending(&first);
    second.tracking = None;
    second.recorded_by = first.recorded_by.clone();
    second.receipt.action = DeadlineAction::SetAttention;
    second.receipt.version = DeadlineReceiptVersion::Legacy;
    resign(&mut second);
    assert!(read_history(&first, &second, false).is_err());
}
