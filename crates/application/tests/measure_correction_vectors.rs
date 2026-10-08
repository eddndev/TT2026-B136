mod measure_correction_vector_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_vector_support;

use application::measure_corrections::{
    measure_administrative_capture_bytes, measure_administrative_record_bytes,
    measure_administrative_review_bytes, measure_administrative_submission_bytes,
};
use measure_correction_vector_support::*;
use measure_decision_vector_support::assert_vector;

#[test]
fn correction_instruction_matches_independent_matxn1_vector() {
    let capture = correction_capture();
    let review = &capture.review;
    assert_vector(
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)
            .unwrap(),
        INSTRUCTION_LEN,
        INSTRUCTION_HEX,
    );
}

#[test]
fn correction_review_matches_independent_mapr1_vector() {
    let capture = correction_capture();
    assert_vector(
        &measure_administrative_review_bytes(&capture.review).unwrap(),
        REVIEW_LEN,
        REVIEW_HEX,
    );
}

#[test]
fn correction_record_matches_independent_marcr1_vector() {
    let capture = correction_capture();
    assert_eq!(capture.records.len(), 1);
    assert_vector(
        &measure_administrative_record_bytes(&capture.records[0]).unwrap(),
        RECORD_LEN,
        RECORD_HEX,
    );
}

#[test]
fn correction_receipt_matches_independent_magr1_vector() {
    let capture = correction_capture();
    assert_vector(
        &measure_administrative_capture_bytes(&capture).unwrap(),
        CAPTURE_LEN,
        CAPTURE_HEX,
    );
}
