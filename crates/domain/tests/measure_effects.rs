#[path = "measure_effect_support/binding.rs"]
mod binding;
#[path = "measure_effect_support/normalization.rs"]
mod normalization;
#[path = "measure_effect_support/mod.rs"]
mod support;
#[path = "measure_effect_support/vectors.rs"]
mod vectors;

use domain::precautionary_measures::{
    MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureEffect,
};
use support::*;

#[test]
fn no_measure_change_preserves_statement_without_affected_identities() {
    let outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note(
        "No change declared",
    )))
    .unwrap();
    assert!(outcome.changes().is_none());
    assert_eq!(
        outcome.no_measure_change(),
        Some(&note("No change declared"))
    );
    assert!(outcome.affected_ids().is_empty());
}

#[test]
fn imposition_retains_complete_proposal() {
    let effect = MeasureEffect::Impose(proposal(1));
    let outcome = changes(vec![effect.clone()]);
    assert_eq!(outcome.changes().unwrap(), &[effect]);
    assert!(outcome.no_measure_change().is_none());
    assert_eq!(outcome.affected_ids(), &[id(1)]);
}
