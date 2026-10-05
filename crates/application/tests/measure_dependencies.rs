#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_decision_review_support/mod.rs"]
mod decision_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
mod precautionary_receipt_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

#[path = "measure_dependency_support/forest.rs"]
mod forest_tests;
#[path = "measure_dependency_support/hearing_edges.rs"]
mod hearing_edge_tests;
mod measure_dependency_support;
#[path = "measure_dependency_support/record_edges.rs"]
mod record_edge_tests;

use measure_dependency_support::*;

#[test]
fn an_initial_record_has_no_known_dependants_in_its_supplied_history() {
    let fixture = Fixture::single();
    let case_id = fixture.case_id;
    let group = fixture.capture();
    let selected = reference(&group.measures[0]);
    let inventory = judicial_inventory(&group);
    let checked =
        inspect_measure_administrative_dependencies(&Hasher, case_id, selected, &inventory)
            .unwrap();

    assert_eq!(checked.case_id(), case_id);
    assert_eq!(checked.target(), selected);
    assert!(checked.dependants().is_empty());
}

#[path = "measure_dependency_support/negative_tests.rs"]
mod negative_tests;
