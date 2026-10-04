#[path = "measure_history_support/bounds.rs"]
mod bounds_tests;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code)]
#[path = "measure_decision_fixtures/mod.rs"]
mod decision_support;
#[path = "measure_history_support/graph.rs"]
mod graph_tests;
mod measure_history_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[path = "measure_history_support/ownership.rs"]
mod ownership_tests;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[path = "measure_history_support/regressions.rs"]
mod regression_tests;
#[path = "measure_history_support/sources.rs"]
mod source_tests;

use decision_support::*;
use measure_history_support::*;

#[test]
fn an_initial_target_resolves_to_its_exact_validated_group_member() {
    let group = Fixture::single().capture();
    let evidence = history(vec![entry(&group, &empty())]);
    let selected = reference(&group.measures[0]);
    let checked =
        resolve_measure_targets(&Hasher, group.review.case_id, &[selected], &evidence).unwrap();
    assert_eq!(checked.targets(), &[owned(&group)]);
}
