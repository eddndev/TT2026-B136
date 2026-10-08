#[allow(dead_code, unused_imports)]
#[path = "measure_decision_vector_support/mod.rs"]
mod decision_vector_support;
mod measure_effect_vector_support;

use application::precautionary_measures::{
    measure_capture_bytes, measure_decision_group_bytes, measure_decision_review_bytes,
};
use decision_vector_support::assert_vector;
use measure_effect_vector_support::*;

#[test]
fn confirmation_review_matches_independent_nonzero_predecessor_vector() {
    let group = later(false);
    assert_vector(
        &measure_decision_review_bytes(&group.review).unwrap(),
        CONFIRM_REVIEW_LEN,
        CONFIRM_REVIEW_HEX,
    );
}

#[test]
fn confirmation_measure_matches_independent_origin_and_predecessor_vector() {
    let group = later(false);
    assert_eq!(group.measures.len(), 1);
    assert_vector(
        &measure_capture_bytes(&group.measures[0]).unwrap(),
        CONFIRM_MEASURE_LEN,
        CONFIRM_MEASURE_HEX,
    );
}

#[test]
fn substitution_group_matches_independent_joint_relationship_vector() {
    let group = later(true);
    assert_eq!(group.measures.len(), 3);
    assert_eq!(group.substitutions.len(), 1);
    assert_vector(
        &measure_decision_group_bytes(&group).unwrap(),
        SUBSTITUTE_GROUP_LEN,
        SUBSTITUTE_GROUP_HEX,
    );
}
