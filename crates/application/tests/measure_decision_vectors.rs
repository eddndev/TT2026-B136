mod measure_decision_vector_support;

use application::precautionary_measures::{
    measure_capture_bytes, measure_decision_capture_bytes, measure_decision_group_bytes,
    measure_decision_review_bytes, measure_decision_submission_bytes,
};
use measure_decision_vector_support::*;

#[test]
fn impose_instruction_matches_independent_mdtxn1_vector() {
    let group = capture(false);
    let review = &group.review;
    assert_vector(
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command).unwrap(),
        IMPOSE_SUBMISSION_LEN,
        IMPOSE_SUBMISSION_HEX,
    );
}

#[test]
fn impose_review_matches_independent_mdpr1_vector() {
    let group = capture(false);
    assert_vector(
        &measure_decision_review_bytes(&group.review).unwrap(),
        IMPOSE_REVIEW_LEN,
        IMPOSE_REVIEW_HEX,
    );
}

#[test]
fn impose_decision_matches_independent_mdcr1_vector() {
    let group = capture(false);
    assert_vector(
        &measure_decision_capture_bytes(&group.decision).unwrap(),
        IMPOSE_DECISION_LEN,
        IMPOSE_DECISION_HEX,
    );
}

#[test]
fn impose_measure_matches_independent_mmcr1_vector() {
    let group = capture(false);
    assert_eq!(group.measures.len(), 1);
    assert_vector(
        &measure_capture_bytes(&group.measures[0]).unwrap(),
        IMPOSE_MEASURE_LEN,
        IMPOSE_MEASURE_HEX,
    );
}

#[test]
fn impose_group_matches_independent_mdgr1_vector() {
    let group = capture(false);
    assert_vector(
        &measure_decision_group_bytes(&group).unwrap(),
        IMPOSE_GROUP_LEN,
        IMPOSE_GROUP_HEX,
    );
}

#[test]
fn no_change_instruction_matches_independent_mdtxn1_vector() {
    let group = capture(true);
    let review = &group.review;
    assert_vector(
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command).unwrap(),
        NO_CHANGE_SUBMISSION_LEN,
        NO_CHANGE_SUBMISSION_HEX,
    );
}

#[test]
fn no_change_review_matches_independent_mdpr1_vector() {
    let group = capture(true);
    assert_vector(
        &measure_decision_review_bytes(&group.review).unwrap(),
        NO_CHANGE_REVIEW_LEN,
        NO_CHANGE_REVIEW_HEX,
    );
}

#[test]
fn no_change_decision_matches_independent_mdcr1_vector() {
    let group = capture(true);
    assert_vector(
        &measure_decision_capture_bytes(&group.decision).unwrap(),
        NO_CHANGE_DECISION_LEN,
        NO_CHANGE_DECISION_HEX,
    );
}

#[test]
fn no_change_group_matches_independent_mdgr1_vector_without_measure_members() {
    let group = capture(true);
    assert!(group.measures.is_empty());
    assert_vector(
        &measure_decision_group_bytes(&group).unwrap(),
        NO_CHANGE_GROUP_LEN,
        NO_CHANGE_GROUP_HEX,
    );
}
