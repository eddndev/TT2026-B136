use super::{proposal_tests::preparation, route_support::*};
use axum::body::{Body, Bytes};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::stream;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn every_valid_maximum_field_fits_when_all_unicode_is_json_escaped() {
    let w = Arc::new(Workflow::default());
    let text = |n| "\u{1f642}".repeat(n);
    let support = json!({"document_id":"11111111-1111-4111-8111-111111111111",
        "version":u32::MAX,"digest":"11".repeat(32),"locator":text(200)});
    let values = json!({"kind":"natural_person","name":{"state":"unidentified","label":text(200),"reason":text(500)},
        "curp":{"state":"unknown","reason":text(500)},"identity_support":support});
    let mut p = preparation();
    p["proposal"]["subject"] =
        json!({"operation":"append","id":SUBJECT,"expected_revision":0,"values":values});
    p["proposal"]["values"]["role"] = json!({"organization":text(200),"legal_status":text(160),
        "profile":{"kind":"victim","contact":{"state":"unknown","reason":text(500)},
            "protection":{"state":"unknown","reason":text(500)}},"role_support":support});
    p["review"]["selection_reason"] = json!(text(500));
    p["review"]["different"]=json!((1..=16).map(|n|json!({
        "candidate":{"kind":"subject","id":uuid::Uuid::from_u128(n).to_string(),"revision":u32::MAX},
        "reason":text(200),"support":support})).collect::<Vec<_>>());
    p["certificate_base64"] = json!(STANDARD.encode(vec![0x11; 16 * 1024]));
    let raw = p.to_string().replace('\u{1f642}', "\\ud83d\\ude42");
    assert!(raw.len() < 256 * 1024);
    assert!(raw.len() > 100 * 1024);
    let response = request(
        &w,
        "POST",
        "participants/proposals/prepare",
        Some("owner"),
        raw,
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(w.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn delayed_trailing_bytes_are_consumed_before_invoking_the_workflow() {
    let w = Arc::new(Workflow::default());
    let observed = w.clone();
    let first = Bytes::from(preparation().to_string());
    let chunks = stream::unfold((0, first), move |(step, first)| {
        let observed = observed.clone();
        async move {
            match step {
                0 => Some((Ok::<_, std::io::Error>(first.clone()), (1, first))),
                1 => {
                    assert!(observed.calls.lock().unwrap().is_empty());
                    tokio::task::yield_now().await;
                    Some((Ok(Bytes::from_static(b" {}")), (2, first)))
                }
                _ => None,
            }
        }
    });
    let response = request(
        &w,
        "POST",
        "participants/proposals/prepare",
        Some("owner"),
        Body::from_stream(chunks),
    )
    .await;
    assert_eq!(response.status().as_u16(), 400);
    assert!(w.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn body_limit_reports_413_even_when_the_prefix_is_valid() {
    let w = Arc::new(Workflow::default());
    let input = format!("{}{}", preparation(), " ".repeat(256 * 1024));
    let response = request(
        &w,
        "POST",
        "participants/proposals/prepare",
        Some("owner"),
        input,
    )
    .await;
    assert_eq!(response.status().as_u16(), 413);
    assert_eq!(
        body(response).await["error"]["code"],
        "typed_participant_body_too_large"
    );
    assert!(w.calls.lock().unwrap().is_empty());
}
