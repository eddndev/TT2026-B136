#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_record_history_support/mod.rs"]
mod record_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_administrative_replacement_support/mod.rs"]
mod replacement_support;

mod measure_records_http_support;
use measure_records_http_support::*;

#[tokio::test]
async fn current_record_read_preserves_genuine_m1_owner_and_complete_proof() {
    let row = initial(1);
    let selected = row.reference;
    let case = row.case_id;
    let mut records = MockRecords::new();
    records
        .expect_get()
        .times(1)
        .return_once(move |token, c, id| {
            assert_eq!((token, c, id), ("staff-token", case, selected.id()));
            Ok(row)
        });
    let (status, body) = request(records, &current_path(selected.id())).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["case_id"], case.to_string());
    assert_eq!(body["reference"], reference_json(selected));
    assert_eq!(body["family"], "m1");
    assert_eq!(body["validity"], "valid");
    assert_eq!(body["record"]["family"], "m1");
    assert_eq!(body["record_root"]["kind"], "judicial");
    assert_eq!(
        body["record_history"]["records"]["judicial"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        body["record_history"]["records"]["administrative"],
        json!([])
    );
    assert_eq!(body["record_history"]["decisions"], json!([]));
}
