#[path = "vector_constants.rs"]
mod vector_constants;

use application::precautionary_hearings::{
    precautionary_hearing_capture_bytes, precautionary_hearing_review_bytes,
};

use crate::precautionary_receipt_support::scheduled;
use vector_constants::{
    CAPTURE_HEX, CAPTURE_LEN, CAPTURE_TEST_DIGEST, REVIEW_HEX, REVIEW_LEN, REVIEW_TEST_DIGEST,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn complete_review_matches_independent_phpr1_golden_vector() {
    let capture = scheduled();
    let bytes = precautionary_hearing_review_bytes(&capture.review).unwrap();

    assert_eq!(bytes.len(), REVIEW_LEN);
    assert_eq!(hex(&bytes), REVIEW_HEX);
    assert_eq!(capture.review.review_digest.to_hex(), REVIEW_TEST_DIGEST);
}

#[test]
fn timestamped_capture_matches_independent_phcr1_golden_vector() {
    let capture = scheduled();
    let bytes = precautionary_hearing_capture_bytes(&capture).unwrap();

    assert_eq!(bytes.len(), CAPTURE_LEN);
    assert_eq!(hex(&bytes), CAPTURE_HEX);
    assert_eq!(capture.capture_digest.to_hex(), CAPTURE_TEST_DIGEST);
}
