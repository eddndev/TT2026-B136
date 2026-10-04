#[path = "decision_anchor_support/context_regressions.rs"]
mod context_regressions;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
mod decision_anchor_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_fixtures/mod.rs"]
mod decision_support;
#[path = "decision_anchor_support/graph.rs"]
mod graph_tests;
#[allow(dead_code)]
#[path = "measure_history_support/mod.rs"]
mod history_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[path = "decision_anchor_support/ordinary.rs"]
mod ordinary_tests;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code)]
#[path = "precautionary_receipt_support/mod.rs"]
mod precautionary_receipt_support;
#[path = "decision_anchor_support/precautionary.rs"]
mod precautionary_tests;

use decision_anchor_support::*;
use decision_support::*;

#[test]
fn an_exact_ordinary_initial_hearing_anchors_a_complete_decision_group() {
    let detail = ordinary_initial();
    application::hearings::hearing_receipt_matches(&Hasher, &detail).unwrap();
    let mut fixture = Fixture::single();
    attach_initial(&mut fixture, detail.clone());
    let group = capture(fixture, &empty(), at());
    assert_eq!(
        group.decision.anchor,
        Some(MeasureDecisionAnchorMaterial::Initial(Box::new(detail)))
    );
    measure_decision_group_with_history_matches(&Hasher, &group, &empty()).unwrap();
}

#[path = "decision_anchor_support/projection_regressions.rs"]
mod projection_regressions;
