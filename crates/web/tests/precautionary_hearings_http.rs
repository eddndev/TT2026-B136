#[allow(dead_code)]
#[path = "../../application/tests/precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code)]
#[path = "../../application/tests/precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code)]
#[path = "../../application/tests/precautionary_receipt_support/mod.rs"]
mod receipt_support;

mod precautionary_hearings_http_support;
use precautionary_hearings_http_support::*;

#[tokio::test]
async fn atomic_context_route_returns_exact_context_and_digest() {
    let context = receipt_support::context();
    let expected = context.digest(&Hasher).to_hex();
    let mut port = MockContext::new();
    port.expect_get().times(1).return_once(move |token, case| {
        assert_eq!(token, "staff-token");
        assert_eq!(case, case_id());
        Ok(context)
    });
    let (status, body) = request(
        port,
        MockWrite::new(),
        MockRead::new(),
        "GET",
        &context_path(),
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["case_id"], case_id().to_string());
    assert_eq!(body["administration"]["revision"], 1);
    assert_eq!(body["stage"]["stage_revision"], 1);
    assert_eq!(body["stage_administration"]["revision"], 1);
    assert_eq!(body["context_digest"], expected);
    assert_eq!(
        body["expectation"],
        serde_json::json!({
            "administration_revision":1,"stage_revision":1,"context_digest":expected,
        })
    );
}
