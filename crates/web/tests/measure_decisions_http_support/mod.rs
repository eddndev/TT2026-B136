#![allow(dead_code)]
pub use crate::context_support::Hasher;
pub use application::{precautionary_measures::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
pub use domain::precautionary_hearings::PrecautionaryMeasureRef;
pub use domain::{cases::CaseId, crypto::Sha256Digest, precautionary_measures::*};
use mockall::mock;
pub use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

mod bindings;
mod body_bounds;
mod commands;
mod fixture;
mod reads;
mod transport;
mod wire;
pub use fixture::*;
pub use wire::*;

mock! {
    pub Write {}
    impl MeasureDecisionRecordWorkflow for Write {
        fn prepare(&self, token:&str, case_id:CaseId, command:MeasureDecisionCommand)->Result<MeasureDecisionRecordReview,ApplicationError>;
        fn submit(&self, token:&str, case_id:CaseId, command:MeasureDecisionCommand, confirmation:MeasureDecisionConfirmation)->Result<MeasureDecisionRecordReceipt,ApplicationError>;
    }
}
mock! {
    pub Read {}
    impl MeasureDecisionRecordReadWorkflow for Read {
        fn list(&self, token:&str, case_id:CaseId, query:MeasureDecisionReadQuery)->Result<MeasureDecisionRecordPage,ApplicationError>;
        fn get(&self, token:&str, case_id:CaseId, decision:MeasureDecisionId)->Result<MeasureDecisionRecordReceipt,ApplicationError>;
        fn get_operation(&self, token:&str, case_id:CaseId, operation:MeasureDecisionOperationId)->Result<MeasureDecisionRecordReceipt,ApplicationError>;
    }
}
pub fn case_id() -> CaseId {
    crate::context_support::initial().case_id
}
pub fn base() -> String {
    format!("/api/v1/cases/{}/measure-decisions", case_id())
}
pub async fn raw(
    ports: (MockWrite, MockRead),
    method: &str,
    path: &str,
    token: &str,
    body: String,
    content_type: &str,
) -> (u16, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", content_type);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response =
        web::measure_decision_router(Arc::new(ports.0), Arc::new(ports.1), Arc::new(Hasher))
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn request(
    write: MockWrite,
    read: MockRead,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    raw(
        (write, read),
        method,
        path,
        "staff-token",
        body.map(|v| v.to_string()).unwrap_or_default(),
        "application/json",
    )
    .await
}
pub fn internal() -> Value {
    json!({"error":{"code":"internal_error","message":"internal application error"}})
}
