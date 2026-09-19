#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;

use application::{
    deadline_evaluations::{evaluate_profiled_deadline, DeadlineEvaluationRecord},
    deadlines::*,
};
use deadline_support::{evaluation::inputs, fixture};
use domain::crypto::Sha256Digest;
use std::fmt::Write;

// Fixed fingerprints of fixtures/deadline_legacy_{review,capture,submission}.hex
// use the deterministic TestHasher in support/document_workflow.rs, not SHA-256.
const LEGACY_REVIEW_DIGEST: &str =
    "23d0a79e47e5a0984f20dfc13f34c79cb5f1d8ba46809393617032f72a9ed47d";
const LEGACY_CAPTURE_DIGEST: &str =
    "346c49454d6d28c980d3805e2b58fdc8068156c85ead213dfacd3c496541f5e0";
const LEGACY_SUBMISSION_DIGEST: &str =
    "97a37d9499cb027ab4610b512b25d172302be5b9e160e1d96f4763a28c71003d";

// Build the historical record explicitly, independently of the current writer.
// The shared fixture supplies only deterministic identities and input material.
fn legacy_v1_fixture() -> (DeadlineCommand, DeadlineDetail) {
    let (command, preparation) = fixture();
    let DeadlineChange::Register { definition } = &command.change else {
        panic!("legacy registration inputs expected");
    };
    let resolved = preparation.resolved.unwrap();
    let evaluation = evaluate_profiled_deadline(
        inputs::hasher().as_ref(),
        &resolved.profile.definition,
        &definition.input,
        &resolved.material,
    )
    .unwrap();
    let value = DeadlineDetail {
        id: command.deadline_id,
        case_id: preparation.case_id,
        revision: DeadlineRevision::new(1).unwrap(),
        definition: definition.clone(),
        tracking: None,
        calculation: DeadlineCalculation {
            profile: resolved.profile,
            material: resolved.material,
            result: DeadlineEvaluationRecord::capture(&evaluation),
        },
        responsible: preparation.responsible.unwrap(),
        attention: DeadlineAttention::Pending,
        status: DeadlineStatus::Active,
        reason: None,
        receipt: DeadlineReceipt {
            version: DeadlineReceiptVersion::Legacy,
            operation_id: command.operation_id,
            action: DeadlineAction::Register,
            expected_revision: 0,
            review_digest: Sha256Digest::from_hex(LEGACY_REVIEW_DIGEST).unwrap(),
            capture_digest: Sha256Digest::from_hex(LEGACY_CAPTURE_DIGEST).unwrap(),
            submission_digest: Sha256Digest::from_hex(LEGACY_SUBMISSION_DIGEST).unwrap(),
        },
        recorded_at: case_support::instant(),
        recorded_by: DeadlineActorSnapshot::User {
            id: inputs::actor(),
            email: "owner@example.com".into(),
        },
    };
    (command, value)
}

fn legacy_bytes() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (command, value) = legacy_v1_fixture();
    let hasher = inputs::hasher();
    deadline_receipt_matches(hasher.as_ref(), &value).unwrap();
    let review = deadline_review_bytes(hasher.as_ref(), &value).unwrap();
    let capture = deadline_capture_bytes(hasher.as_ref(), &value).unwrap();
    let submission = deadline_submission_bytes(
        value.recorded_by.user_id().unwrap(),
        value.case_id,
        &command,
        value.receipt.review_digest,
    );
    assert_eq!(&review[..5], b"DLRV1");
    assert_eq!(&capture[..5], b"DLST1");
    assert_eq!(&submission[..5], b"DLTX1");
    assert_eq!(hasher.hash_bytes(&review), value.receipt.review_digest);
    assert_eq!(hasher.hash_bytes(&capture), value.receipt.capture_digest);
    assert_eq!(
        hasher.hash_bytes(&submission),
        value.receipt.submission_digest
    );
    assert_ne!(review, capture);
    (review, capture, submission)
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").unwrap();
    }
    encoded
}

#[test]
fn legacy_v1_reference_bytes_are_deterministic_and_match_the_receipt() {
    let (review, capture, submission) = legacy_bytes();
    assert_eq!(
        (review.clone(), capture.clone(), submission.clone()),
        legacy_bytes(),
    );
    assert_eq!(
        hex(&review),
        include_str!("fixtures/deadline_legacy_review.hex").trim()
    );
    assert_eq!(
        hex(&capture),
        include_str!("fixtures/deadline_legacy_capture.hex").trim()
    );
    assert_eq!(
        hex(&submission),
        include_str!("fixtures/deadline_legacy_submission.hex").trim()
    );
}
