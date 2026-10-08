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
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

mod measure_decision_record_read_support;
use measure_decision_record_read_support::*;

#[test]
fn mixed_decision_reads_retain_genuine_original_v1_and_v2_receipts() {
    let actor = reader(Role::Paralegal);
    for original in [v1(10), v2(10)] {
        for kind in READS {
            let store = successful_store(&actor, &original, original.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor)), kind, &original).unwrap(),
                vec![original.clone()]
            );
        }
    }
}
