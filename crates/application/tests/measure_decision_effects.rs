#[path = "measure_decision_effect_support/ancestry_boundary.rs"]
mod ancestry_boundary_tests;
#[path = "measure_decision_effect_support/boundaries.rs"]
mod boundary_tests;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
mod measure_decision_effect_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[path = "measure_decision_effect_support/source_continuity.rs"]
mod source_continuity_tests;
#[path = "measure_decision_effect_support/substitution.rs"]
mod substitution_tests;
#[path = "measure_decision_effect_support/transitions.rs"]
mod transition_tests;

use measure_decision_effect_support::*;

#[test]
fn confirmation_appends_an_exact_revision_with_full_retained_terms_sources_and_origin() {
    let previous = Fixture::single().capture();
    let request = LaterFixture::confirm(&previous);
    let evidence = request.evidence.clone();
    let group = request.capture();
    let original = &previous.measures[0];
    let confirmed = &group.measures[0];
    assert_eq!(group.measures.len(), 1);
    assert!(group.substitutions.is_empty());
    assert_eq!(confirmed.result.id, original.result.id);
    assert_eq!(confirmed.result.revision, MeasureRevision::new(2).unwrap());
    assert_eq!(confirmed.result.origin, original.result.origin);
    assert_eq!(confirmed.result.values, original.result.values);
    assert_eq!(confirmed.result.sources, original.result.sources);
    assert_eq!(confirmed.result.projection, original.result.projection);
    assert_eq!(confirmed.result.previous, Some(reference(original)));
    assert_eq!(confirmed.result.action, MeasureCaptureAction::Confirm);
    assert_eq!(confirmed.result.effect_key, original.result.id);
    assert_eq!(confirmed.operation_id, group.decision.operation_id);
    assert_eq!(confirmed.decision_id, group.decision.decision_id);
    assert_eq!(confirmed.decision_digest, group.decision.capture_digest);
    assert_ne!(confirmed.decision_id, original.decision_id);
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
    assert!(measure_decision_group_matches(&Hasher, &group).is_err());
}
