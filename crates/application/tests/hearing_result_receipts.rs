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
use domain::{cases::CaseId, identity::UserId};
use hearing_result_support::*;
#[test]
fn self_contained_receipt_detects_scope_action_sources_and_value_tampering() {
    let actor = UserId::new();
    let prep = preparation(CaseId::new(), actor);
    let base = detail(actor, &command(&prep), &prep);
    hearing_result_receipt_matches(hasher().as_ref(), &base).unwrap();
    let entry = HearingResultHistoryEntry::from(&base.snapshot);
    hearing_result_history_receipt_matches(hasher().as_ref(), &entry).unwrap();
    for mode in 0..12 {
        let mut changed = base.clone();
        match mode {
            0 => changed.snapshot.case_id = CaseId::new(),
            1 => changed.snapshot.recorded_by.id = UserId::new(),
            2 => changed.snapshot.id = HearingResultId::new(),
            3 => changed.snapshot.hearing_id = domain::hearings::HearingId::new(),
            4 => changed.snapshot.revision = HearingResultRevision::new(2).unwrap(),
            5 => changed.snapshot.receipt.operation_id = HearingResultOperationId::new(),
            6 => changed.snapshot.receipt.expected_revision = 1,
            7 => changed.snapshot.status = HearingResultStatus::Withdrawn,
            8 => changed.snapshot.reason = Some(HearingResultText::new("Injected").unwrap()),
            9 => changed.snapshot.values_digest = domain::crypto::Sha256Digest::from_array([0; 32]),
            10 => {
                changed.snapshot.anchor.revision =
                    domain::hearings::HearingRevision::new(2).unwrap()
            }
            _ => {
                changed.anchor.reference.submission_digest =
                    domain::crypto::Sha256Digest::from_array([0; 32])
            }
        }
        assert!(
            matches!(
                hearing_result_receipt_matches(hasher().as_ref(), &changed),
                Err(ApplicationError::HearingResult(
                    HearingResultError::StoredInconsistent(_)
                ))
            ),
            "mode {mode}"
        );
    }
}
#[test]
fn withdrawal_preserves_values_even_when_capture_clock_moves_backwards() {
    let actor = UserId::new();
    let mut prep = preparation(CaseId::new(), actor);
    let base = detail(actor, &command(&prep), &prep);
    let cmd = withdrawal(&base);
    prep.base = Some(base);
    let mut withdrawn = detail(actor, &cmd, &prep);
    withdrawn.snapshot.recorded_at = instant() - time::Duration::days(5);
    hearing_result_receipt_matches(hasher().as_ref(), &withdrawn).unwrap();
    let entry = HearingResultHistoryEntry::from(&withdrawn.snapshot);
    hearing_result_history_receipt_matches(hasher().as_ref(), &entry).unwrap();
}
#[test]
fn recorded_time_must_not_precede_declared_day_or_instant() {
    let actor = UserId::new();
    let prep = preparation(CaseId::new(), actor);
    let mut result = detail(actor, &command(&prep), &prep);
    result.snapshot.recorded_at = instant() - time::Duration::days(1);
    assert!(hearing_result_receipt_matches(hasher().as_ref(), &result).is_err());
}
