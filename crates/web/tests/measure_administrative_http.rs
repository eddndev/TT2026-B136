#[allow(dead_code)]
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
#[allow(dead_code)]
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

mod measure_administrative_http_support;
use measure_administrative_http_support::*;

#[tokio::test]
async fn correct_prepare_submit_and_operation_read_preserve_original_receipt() {
    let operation = correct();
    let review = operation.capture.review.clone();
    let command = command_json(&review);
    let mut write = MockWrite::new();
    let expected = review.clone();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |token, case, command| {
            assert_eq!(token, "staff-token");
            assert_eq!(case, expected.case_id);
            assert_eq!(command, expected.command);
            Ok(expected)
        });
    let (status, body) = request(
        write,
        MockRead::new(),
        "POST",
        &prepare_path(),
        Some(command),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["command"], command_json(&review));
    assert_eq!(body["submission_digest"], review.submission_digest.to_hex());
    assert_eq!(body["review_digest"], review.review_digest.to_hex());
    assert_eq!(body["result"]["validity"], "valid");
    assert!(body["replacement"].is_null());

    let mut write = MockWrite::new();
    let expected = operation.clone();
    write
        .expect_submit()
        .times(1)
        .return_once(move |token, case, command, confirmation| {
            assert_eq!(token, "staff-token");
            assert_eq!(case, expected.capture.review.case_id);
            assert_eq!(command, expected.capture.review.command);
            assert_eq!(confirmation, confirm(&expected));
            Ok(expected)
        });
    let (status, submitted) = request(
        write,
        MockRead::new(),
        "POST",
        &submit_path(),
        Some(submit_json(&operation)),
    )
    .await;
    assert_eq!(status, 201, "{submitted}");
    assert_stored(&submitted, &operation);

    let mut read = MockRead::new();
    let expected = operation.clone();
    read.expect_get_operation()
        .times(1)
        .return_once(move |token, case, id| {
            assert_eq!(token, "staff-token");
            assert_eq!(case, expected.capture.review.case_id);
            assert_eq!(id, expected.origin.operation_id);
            Ok(expected)
        });
    let (status, recovered) = request(
        MockWrite::new(),
        read,
        "GET",
        &operation_path(&operation),
        None,
    )
    .await;
    assert_eq!(status, 200, "{recovered}");
    assert_eq!(recovered, submitted);
}
