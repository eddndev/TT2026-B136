#![allow(dead_code)]
pub use crate::context_support::Hasher;
pub use application::{precautionary_hearings::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
pub use domain::{cases::CaseId, crypto::Sha256Digest, precautionary_hearings::*};
use mockall::mock;
pub use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

mod fixture;
mod history_bounds;
pub use fixture::*;
mod commands;
mod context_errors;
mod prefix_regressions;
mod reads;
mod transport;

mock! {
    pub Context {}
    impl PrecautionaryContextReadWorkflow for Context {
        fn get(&self, token:&str, case_id:CaseId)->Result<PrecautionaryContext,ApplicationError>;
    }
}
mock! {
    pub Write {}
    impl PrecautionaryHearingRecordWorkflow for Write {
        fn prepare(&self, token:&str, case_id:CaseId, command:PrecautionaryHearingCommand)->Result<PrecautionaryHearingReview,ApplicationError>;
        fn submit(&self, token:&str, case_id:CaseId, command:PrecautionaryHearingCommand, confirmation:PrecautionaryHearingConfirmation)->Result<PrecautionaryHearingRecordStoredOperation,ApplicationError>;
    }
}
mock! {
    pub Read {}
    impl PrecautionaryHearingRecordReadWorkflow for Read {
        fn list(&self, token:&str, case_id:CaseId, query:PrecautionaryHearingReadQuery)->Result<PrecautionaryHearingRecordPage,ApplicationError>;
        fn get(&self, token:&str, case_id:CaseId, hearing:PrecautionaryHearingId, revision:Option<PrecautionaryHearingRevision>)->Result<PrecautionaryHearingRecordStoredOperation,ApplicationError>;
        fn get_operation(&self, token:&str, case_id:CaseId, operation:PrecautionaryHearingOperationId)->Result<PrecautionaryHearingRecordStoredOperation,ApplicationError>;
    }
}
pub fn case_id() -> CaseId {
    crate::context_support::initial().case_id
}
pub fn base() -> String {
    format!("/api/v1/cases/{}/precautionary-hearings", case_id())
}
pub fn context_path() -> String {
    format!("/api/v1/cases/{}/precautionary-context", case_id())
}
pub async fn raw(
    ports: (MockContext, MockWrite, MockRead),
    method: &str,
    path: &str,
    token: &str,
    body: String,
    content_type: &str,
) -> (u16, Value) {
    let (context, write, read) = ports;
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", content_type);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = web::precautionary_hearing_router(
        Arc::new(context),
        Arc::new(write),
        Arc::new(read),
        Arc::new(Hasher),
    )
    .oneshot(request.body(Body::from(body)).unwrap())
    .await
    .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn request(
    context: MockContext,
    write: MockWrite,
    read: MockRead,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    raw(
        (context, write, read),
        method,
        path,
        "staff-token",
        body.map(|b| b.to_string()).unwrap_or_default(),
        "application/json",
    )
    .await
}
pub fn internal() -> Value {
    json!({"error":{"code":"internal_error","message":"internal application error"}})
}
