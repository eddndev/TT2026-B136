#[allow(dead_code)]
mod case_support;
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
#[path = "precautionary_hearing_record_read_support/mod.rs"]
mod read_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

use read_support::*;

#[test]
fn mixed_hearing_reads_return_the_original_m2_capture_and_full_actual_owner_history() {
    let saved = operation(40);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &saved, saved.clone(), kind);
        let result = read(&service(store, identity(&actor), clock()), &saved, kind).unwrap();
        assert_eq!(result, vec![saved.clone()]);
        assert_eq!(result[0].history.record_history.decisions.len(), 1);
        assert_eq!(
            result[0]
                .history
                .record_history
                .records
                .administrative
                .len(),
            1
        );
        assert_eq!(
            result[0]
                .history
                .record_history
                .records
                .judicial
                .groups
                .len(),
            1
        );
        assert_ne!(result[0].capture.review.actor, actor);
    }
}
