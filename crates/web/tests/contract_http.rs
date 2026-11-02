//! Contract routes over the in-memory contract doubles:
//! docs/adr/0086-bilateral-contract-signing.md.
#[allow(dead_code)]
#[path = "../../application/tests/case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../../application/tests/contract_support/mod.rs"]
mod contract_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../../application/tests/document_signing_support/mod.rs"]
mod document_signing_support;

use application::{contracts::*, identity::Principal};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use contract_support::{client, firm, store::*, FakeStore, Harness};
use domain::identity::Role;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use EventKind::*;

const CASE: &str = "/api/v1/cases/00000000-0000-0000-0000-000000000003/contracts";
const CONTRACT: &str = "00000000-0000-0000-0000-000000000032";
const OPERATION: &str = "00000000-0000-0000-0000-000000001b58";

fn harness(principal: Principal, kinds: &[EventKind]) -> Harness {
    let mut harness = Harness::new(principal);
    harness.store = Arc::new(FakeStore::with(snapshot(kinds)));
    harness
}

fn app(harness: &Harness) -> Router {
    web::contract_router(
        web::ContractHttp {
            service: Arc::new(harness.service()),
        },
        web::HttpLimits::default(),
    )
}

fn request_path(suffix: &str) -> String {
    format!(
        "/api/v1/contract-requests/{}{suffix}",
        Uuid::from_u128(REQUEST)
    )
}

async fn call(app: &Router, method: &str, path: &str, body: Value) -> (u16, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", "Bearer session");
    let bytes = if body.is_null() {
        Vec::new()
    } else {
        request = request.header("content-type", "application/json");
        serde_json::to_vec(&body).unwrap()
    };
    let response = app
        .clone()
        .oneshot(request.body(Body::from(bytes)).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    if bytes.is_empty() {
        return (status, Value::Null);
    }
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn prepared(app: &Router) -> Value {
    let (status, body) = call(
        app,
        "POST",
        &request_path("/prepare"),
        json!({ "operation_id": OPERATION }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body
}

fn signing(prepared: &Value) -> Value {
    json!({"operation_id":OPERATION,"expected_revision":prepared["revision"],
        "statement_base64":prepared["statement_base64"],
        "signature_base64":STANDARD.encode([7u8; 384])})
}

#[tokio::test]
async fn the_client_prepares_signs_and_recovers_the_exact_receipt() {
    let harness = harness(client(), &[Requested]);
    let app = app(&harness);
    let prepared = prepared(&app).await;
    assert_eq!(prepared["position"], 1);
    assert_eq!(prepared["revision"], 2);
    assert_eq!(prepared["account_id"], client().id.to_string());
    let raw = STANDARD
        .decode(prepared["statement_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(raw.len(), 303);
    assert!(prepared["client_signature_sha256"].is_null());
    let signed = call(&app, "POST", &request_path("/sign"), signing(&prepared)).await;
    assert_eq!(signed.0, 200, "{}", signed.1);
    assert_eq!(signed.1["event"]["kind"], "client_signed");
    assert_eq!(
        signed.1["signature"]["statement_base64"],
        prepared["statement_base64"]
    );
    assert!(signed.1["signature"]["timestamp_base64"].is_string());
    let replay = call(&app, "POST", &request_path("/sign"), signing(&prepared)).await;
    assert_eq!(replay, signed);
    let recovered = call(
        &app,
        "GET",
        &request_path(&format!("/operations/{OPERATION}")),
        json!(null),
    )
    .await;
    assert_eq!(recovered, signed);
    assert_eq!(harness.calls().2, 1);
}

#[tokio::test]
async fn a_party_reads_its_requests_with_both_certificates_and_the_request_bytes() {
    let harness = harness(client(), &[Requested]);
    let app = app(&harness);
    let mine = call(&app, "GET", "/api/v1/contract-requests", json!(null)).await;
    assert_eq!(mine.0, 200);
    let rows = mine.1["requests"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "awaiting_client");
    assert_eq!(rows[0]["contract_state"], "en_firma");
    let detail = call(&app, "GET", &request_path(""), json!(null)).await;
    assert_eq!(detail.0, 200);
    let bytes = STANDARD
        .decode(detail.1["statement_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(bytes.len(), 240);
    assert_eq!(&bytes[..8], b"CTRTREQ1");
    assert_eq!(detail.1["revision"], 2);
    assert!(detail.1["client_certificate"]["der_base64"].is_string());
    assert!(detail.1["firm_certificate"]["der_base64"].is_string());
    assert_eq!(detail.1["signatures"], json!([]));
}

#[tokio::test]
async fn staff_routes_list_detail_and_reach_distinct_receipt_routes() {
    let harness = harness(firm(), &[Requested]);
    let app = app(&harness);
    let list = call(&app, "GET", CASE, json!(null)).await;
    assert_eq!(list.0, 200);
    assert_eq!(list.1["contracts"][0]["contract_id"], CONTRACT);
    assert_eq!(list.1["contracts"][0]["state"], "en_firma");
    assert_eq!(list.1["firm_signers"][0]["role"], "litigator");
    let detail = call(&app, "GET", &format!("{CASE}/{CONTRACT}"), json!(null)).await;
    assert_eq!(detail.0, 200);
    assert_eq!(detail.1["events"].as_array().unwrap().len(), 2);
    assert_eq!(detail.1["requests"][0]["status"], "awaiting_client");
    let missing = call(
        &app,
        "GET",
        &format!("{CASE}/operations/{OPERATION}"),
        json!(null),
    )
    .await;
    assert_eq!(missing.0, 404);
    assert_eq!(missing.1["error"]["code"], "contract_not_found");
}

#[tokio::test]
async fn an_ineligible_request_names_only_its_blockers_and_writes_nothing() {
    let harness = harness(firm(), &[]);
    harness.store.state().candidates = Some(RequestCandidates {
        client: None,
        firm_signer: Some((
            firm().id,
            Role::Litigator,
            document_signing_support::receipt(),
        )),
        document: None,
        trust: Some(document_signing_support::trust()),
    });
    let app = app(&harness);
    let body = json!({"request_id":Uuid::new_v4().to_string(),"operation_id":OPERATION,
        "expected_revision":1,"version":1,"client_email":"client@example.test",
        "firm_signer_id":firm().id.to_string(),"ttl_days":7,"ceremony_reference":"HF-1"});
    let result = call(&app, "POST", &format!("{CASE}/{CONTRACT}/requests"), body).await;
    assert_eq!(result.0, 422);
    assert_eq!(result.1["error"]["code"], "contract_party_ineligible");
    assert_eq!(
        result.1["error"]["blockers"],
        json!(["client_not_ready", "document_not_ready"])
    );
    assert_eq!(harness.calls().2, 0);
}

#[tokio::test]
async fn refusals_keep_their_contract_and_signing_codes() {
    let harness = harness(client(), &[Requested]);
    harness.set(|world| world.tsa_failure = true);
    let app = app(&harness);
    let prepared = prepared(&app).await;
    let result = call(&app, "POST", &request_path("/sign"), signing(&prepared)).await;
    assert_eq!(
        (result.0, result.1["error"]["code"].clone()),
        (503, json!("contract_timestamp_unavailable"))
    );
    let mut stale = signing(&prepared);
    stale["expected_revision"] = json!(1);
    harness.set(|world| world.tsa_failure = false);
    let result = call(&app, "POST", &request_path("/sign"), stale).await;
    assert_eq!(result.1["error"]["code"], "contract_revision_conflict");
    let declined = call(
        &app,
        "POST",
        &request_path("/close"),
        json!({"operation_id":OPERATION,"expected_revision":2,"outcome":"decline"}),
    )
    .await;
    assert_eq!(declined.0, 403);
    let early = call(&app, "GET", &request_path("/evidence/final"), json!(null)).await;
    assert_eq!(early.1["error"]["code"], "contract_not_complete");
    harness.store.state().snapshot = None;
    let hidden = call(&app, "GET", &request_path(""), json!(null)).await;
    assert_eq!(
        (hidden.0, hidden.1["error"]["code"].clone()),
        (404, json!("contract_not_found"))
    );
    assert_eq!(harness.calls().2, 0);
}

#[tokio::test]
async fn the_requester_cancels_with_a_reason_and_malformed_input_is_refused() {
    let harness = harness(firm(), &[Requested]);
    let app = app(&harness);
    for (path, body) in [
        (
            request_path("/close"),
            json!({"operation_id":OPERATION,"expected_revision":2,"outcome":"cancel"}),
        ),
        (
            request_path("/close"),
            json!({"operation_id":OPERATION,"expected_revision":2,"outcome":"other","reason":"x"}),
        ),
        (
            request_path("/prepare"),
            json!({"operation_id":OPERATION,"extra":true}),
        ),
        (request_path("/prepare"), json!([OPERATION])),
        (
            request_path("/prepare?x=1"),
            json!({"operation_id":OPERATION}),
        ),
        (
            "/api/v1/contract-requests/not-a-uuid/prepare".into(),
            json!({"operation_id":OPERATION}),
        ),
        (
            request_path("/sign"),
            json!({"operation_id":OPERATION,"expected_revision":2,
            "statement_base64":"AQ==","signature_base64":"AQ=="}),
        ),
    ] {
        let result = call(&app, "POST", &path, body).await;
        assert_eq!(result.0, 400, "{path} {}", result.1);
    }
    let stage = call(&app, "GET", &request_path("/evidence/draft"), json!(null)).await;
    assert_eq!(stage.1["error"]["code"], "invalid_contract_request");
    let head = call(&app, "HEAD", &request_path("/content"), json!(null)).await;
    assert_eq!(head.0, 405);
    assert_eq!(harness.calls().2, 0);
    let cancelled = call(
        &app,
        "POST",
        &request_path("/close"),
        json!({"operation_id":OPERATION,
        "expected_revision":2,"outcome":"cancel","reason":"Cambio de condiciones"}),
    )
    .await;
    assert_eq!(cancelled.0, 200, "{}", cancelled.1);
    assert_eq!(cancelled.1["event"]["kind"], "cancelled");
    assert_eq!(harness.calls().2, 1);
}
