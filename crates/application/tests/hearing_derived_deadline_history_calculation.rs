use crate::hearing_derived_deadline_history::captured_creation;
use crate::hearing_result_support::hasher;
use application::{
    deadline_evaluations::{
        deadline_evaluation_record_bytes, decode_deadline_evaluation_record,
        evaluate_profiled_deadline,
    },
    deadlines::{
        deadline_capture_bytes, deadline_receipt_matches, deadline_record_submission_bytes,
        deadline_review_bytes,
    },
    hearing_derived_deadlines::restore_hearing_derived_deadline,
};
use time::Duration;

#[test]
fn reading_history_preserves_a_valid_captured_result_instead_of_recalculating_it() {
    let original = captured_creation(false).evidence();
    let record = restore_hearing_derived_deadline(hasher().as_ref(), original.clone()).unwrap();
    let mut evidence = original.clone();
    let original_result = &original.deadline.calculation.result;
    let due = original_result.due_at().unwrap();
    let synthetic_due = due + Duration::seconds(1);
    let mut result_bytes = deadline_evaluation_record_bytes(original_result);

    // Synthetic DRES1 fixture: the civil trace is unchanged, but its captured
    // closing instant differs from today's evaluator. This is not a legal rule
    // or a result observed in production. The real codec must accept its shape.
    let end = result_bytes.len();
    assert_eq!(&result_bytes[end - 4..], &0_u32.to_be_bytes());
    assert_eq!(result_bytes[end - 21], 1);
    assert_eq!(
        &result_bytes[end - 20..end - 12],
        &due.unix_timestamp().to_be_bytes()
    );
    result_bytes[end - 20..end - 12].copy_from_slice(&synthetic_due.unix_timestamp().to_be_bytes());
    let historical_result = decode_deadline_evaluation_record(&result_bytes).unwrap();
    assert_eq!(
        deadline_evaluation_record_bytes(&historical_result),
        result_bytes
    );
    assert_eq!(historical_result.arithmetic(), original_result.arithmetic());
    assert_eq!(historical_result.due_at(), Some(synthetic_due));
    evidence.deadline.calculation.result = historical_result.clone();

    // Reproduce the ordinary receipt using the existing captured-state codecs.
    let digest = hasher();
    evidence.deadline.receipt.review_digest =
        digest.hash_bytes(&deadline_review_bytes(digest.as_ref(), &evidence.deadline).unwrap());
    evidence.deadline.receipt.capture_digest =
        digest.hash_bytes(&deadline_capture_bytes(digest.as_ref(), &evidence.deadline).unwrap());
    evidence.deadline.receipt.submission_digest =
        digest.hash_bytes(&deadline_record_submission_bytes(&evidence.deadline).unwrap());
    deadline_receipt_matches(digest.as_ref(), &evidence.deadline).unwrap();

    // HRDL1 ends with the exact historical result digest. Rebind it without
    // constructing a ProfiledDeadlineEvaluation or asking a finalizer to run.
    let old_result_digest = digest.hash_bytes(&deadline_evaluation_record_bytes(original_result));
    let new_result_digest = digest.hash_bytes(&result_bytes);
    let mut expected_review = record.review_bytes().to_vec();
    let review_end = expected_review.len();
    assert_eq!(
        &expected_review[review_end - 32..],
        old_result_digest.as_bytes()
    );
    expected_review[review_end - 32..].copy_from_slice(new_result_digest.as_bytes());
    evidence.review_digest = digest.hash_bytes(&expected_review);

    // HRDC1 binds the review followed by ordinary submission and capture.
    // Every replacement must identify exactly one existing fixed-width field.
    let mut expected_capture = record.capture_bytes().to_vec();
    assert_eq!(&expected_capture[5..37], original.review_digest.as_bytes());
    expected_capture[5..37].copy_from_slice(evidence.review_digest.as_bytes());
    replace_once(
        &mut expected_capture,
        &deadline_record_submission_bytes(&original.deadline).unwrap(),
        &deadline_record_submission_bytes(&evidence.deadline).unwrap(),
    );
    replace_once(
        &mut expected_capture,
        original.deadline.receipt.submission_digest.as_bytes(),
        evidence.deadline.receipt.submission_digest.as_bytes(),
    );
    replace_once(
        &mut expected_capture,
        original.deadline.receipt.capture_digest.as_bytes(),
        evidence.deadline.receipt.capture_digest.as_bytes(),
    );
    evidence.capture_digest = digest.hash_bytes(&expected_capture);

    let current = evaluate_profiled_deadline(
        digest.as_ref(),
        &evidence.deadline.calculation.profile.definition,
        &evidence.deadline.definition.input,
        &evidence.deadline.calculation.material,
    )
    .unwrap();
    assert_eq!(current.due_at(), Some(due));
    assert_ne!(current.due_at(), historical_result.due_at());

    let restored = restore_hearing_derived_deadline(digest.as_ref(), evidence.clone()).unwrap();
    assert_eq!(restored.evidence(), &evidence);
    assert_eq!(
        restored.evidence().deadline.calculation.result,
        historical_result
    );
    assert_eq!(restored.review_bytes(), expected_review.as_slice());
    assert_eq!(restored.capture_bytes(), expected_capture.as_slice());
    assert_eq!(
        restored.evidence().deadline.operational_due_at(),
        Some(synthetic_due)
    );
}

fn replace_once(bytes: &mut [u8], old: &[u8], new: &[u8]) {
    assert!(!old.is_empty());
    assert_eq!(old.len(), new.len());
    let positions: Vec<_> = bytes
        .windows(old.len())
        .enumerate()
        .filter_map(|(index, candidate)| (candidate == old).then_some(index))
        .collect();
    assert_eq!(positions.len(), 1, "expected one exact encoded field");
    let start = positions[0];
    bytes[start..start + old.len()].copy_from_slice(new);
}
