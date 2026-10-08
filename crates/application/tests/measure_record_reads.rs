#[allow(dead_code)]
mod case_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
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
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

mod measure_record_read_support;
use measure_record_read_support::*;

#[path = "measure_record_read_support/negative_tests.rs"]
mod negative_tests;

#[test]
fn current_exact_and_list_reads_return_the_real_record_and_complete_owning_closure() {
    let original = initial(10);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &original, original.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &original).unwrap(),
            vec![original.clone()]
        );
    }
}
