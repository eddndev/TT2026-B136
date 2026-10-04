#[allow(dead_code, unused_imports)]
#[path = "measure_decision_vector_support/mod.rs"]
mod decision_vector_support;
mod measure_anchor_vector_support;

use application::precautionary_measures::{
    measure_decision_capture_bytes, measure_decision_submission_bytes,
};
use decision_vector_support::assert_vector;
use measure_anchor_vector_support::*;

#[test]
fn initial_anchor_reference_matches_independent_instruction_vector() {
    let (command, decision) = anchored(true);
    assert_vector(
        &measure_decision_submission_bytes(&decision.actor, decision.case_id, &command).unwrap(),
        INITIAL_INSTRUCTION_LEN,
        INITIAL_INSTRUCTION_HEX,
    );
}

#[test]
fn initial_anchor_material_matches_independent_decision_vector() {
    let (_, decision) = anchored(true);
    assert_vector(
        &measure_decision_capture_bytes(&decision).unwrap(),
        INITIAL_DECISION_LEN,
        INITIAL_DECISION_HEX,
    );
}

#[test]
fn precautionary_anchor_reference_matches_independent_instruction_vector() {
    let (command, decision) = anchored(false);
    assert_vector(
        &measure_decision_submission_bytes(&decision.actor, decision.case_id, &command).unwrap(),
        PRECAUTIONARY_INSTRUCTION_LEN,
        PRECAUTIONARY_INSTRUCTION_HEX,
    );
}

#[test]
fn precautionary_anchor_material_matches_independent_decision_vector() {
    let (_, decision) = anchored(false);
    assert_vector(
        &measure_decision_capture_bytes(&decision).unwrap(),
        PRECAUTIONARY_DECISION_LEN,
        PRECAUTIONARY_DECISION_HEX,
    );
}
