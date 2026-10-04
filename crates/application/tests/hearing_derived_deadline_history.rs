use crate::hearing_derived_deadline_capture_support::{recorded_source, source_event};
use crate::hearing_derived_deadline_support::fixture;
use crate::hearing_result_support::hasher;
use application::{
    deadline_evaluations::DeadlineEvaluationBlock, deadlines::deadline_receipt_matches,
    hearing_derived_deadlines::*, hearing_results::hearing_result_receipt_matches,
};
use domain::deadline_profiles::DeadlineRuleBlock;
use time::Duration;

pub fn captured_creation(blocked: bool) -> HearingDerivedDeadlineCreation {
    let mut fixture = fixture();
    if blocked {
        fixture.edit_definition(|value| value.input.ordered_quantity = None);
    }
    let reviewed = fixture.prepare().unwrap();
    let result = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    let event = source_event(&result);
    finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, result, event).unwrap()
}

#[test]
fn historical_evidence_restores_the_original_review_capture_and_ordinary_receipts() {
    let creation = captured_creation(false);
    let evidence = creation.evidence();
    let expected_capture = hearing_derived_deadline_capture_bytes(&creation).unwrap();
    let record = restore_hearing_derived_deadline(hasher().as_ref(), evidence.clone()).unwrap();
    assert_eq!(record.evidence(), &evidence);
    assert!(record.review_bytes().starts_with(b"HRDL1"));
    assert!(record.capture_bytes().starts_with(b"HRDC1"));
    assert_eq!(record.capture_bytes(), expected_capture.as_slice());
    assert_eq!(
        hasher().hash_bytes(record.review_bytes()),
        evidence.review_digest
    );
    assert_eq!(
        hasher().hash_bytes(record.capture_bytes()),
        evidence.capture_digest
    );
    assert_eq!(
        record.evidence().result.snapshot.receipt,
        creation.result().snapshot.receipt
    );
    assert_eq!(
        record.evidence().deadline.receipt,
        creation.deadline().receipt
    );
    hearing_result_receipt_matches(hasher().as_ref(), &record.evidence().result).unwrap();
    deadline_receipt_matches(hasher().as_ref(), &record.evidence().deadline).unwrap();
}

#[test]
fn historical_blocked_result_preserves_its_reason_without_an_operational_due_time() {
    let evidence = captured_creation(true).evidence();
    let record = restore_hearing_derived_deadline(hasher().as_ref(), evidence.clone()).unwrap();
    assert_eq!(record.evidence(), &evidence);
    assert_eq!(record.evidence().deadline.operational_due_at(), None);
    assert_eq!(
        record.evidence().deadline.calculation.result.blocks(),
        &[DeadlineEvaluationBlock::Rule(
            DeadlineRuleBlock::MissingOrderedQuantity
        )],
    );
    assert_eq!(
        hasher().hash_bytes(record.review_bytes()),
        evidence.review_digest
    );
    assert_eq!(
        hasher().hash_bytes(record.capture_bytes()),
        evidence.capture_digest
    );
}
