pub use crate::context_support::Hasher;
pub use application::{measure_corrections::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
pub use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
    precautionary_measures::MeasureCorrectionOperationId,
};
use mockall::mock;
pub use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

mod actions;
mod bindings;
mod errors;
mod fixture;
mod reads;
mod time_size;
mod transport;
pub use fixture::*;

mock! {
    pub Write {}
    impl MeasureAdministrativeWorkflow for Write {
        fn prepare(&self, token:&str, case_id:CaseId, command:MeasureAdministrativeCommand)->Result<MeasureAdministrativeReview,ApplicationError>;
        fn submit(&self, token:&str, case_id:CaseId, command:MeasureAdministrativeCommand, confirmation:MeasureAdministrativeConfirmation)->Result<MeasureAdministrativeStoredOperation,ApplicationError>;
    }
}
mock! {
    pub Read {}
    impl MeasureAdministrativeReadWorkflow for Read {
        fn list(&self, token:&str, case_id:CaseId, query:MeasureAdministrativeReadQuery)->Result<MeasureAdministrativePage,ApplicationError>;
        fn get_operation(&self, token:&str, case_id:CaseId, operation:MeasureCorrectionOperationId)->Result<MeasureAdministrativeStoredOperation,ApplicationError>;
    }
}
mock! {
    pub Records {}
    impl MeasureRecordReadWorkflow for Records {
        fn list(&self, token:&str, case_id:CaseId, query:MeasureRecordReadQuery)->Result<MeasureRecordPage,ApplicationError>;
        fn get(&self, token:&str, case_id:CaseId, id:MeasureId)->Result<MeasureRecordDetail,ApplicationError>;
        fn exact(&self, token:&str, case_id:CaseId, reference:PrecautionaryMeasureRef)->Result<MeasureRecordDetail,ApplicationError>;
    }
}
pub fn case_id() -> CaseId {
    crate::context_support::initial().case_id
}
pub fn base() -> String {
    format!(
        "/api/v1/cases/{}/measure-administrative-operations",
        case_id()
    )
}
pub fn prepare_path() -> String {
    format!("{}/prepare", base())
}
pub fn submit_path() -> String {
    format!("{}/submit", base())
}
pub fn operation_path(value: &MeasureAdministrativeStoredOperation) -> String {
    format!("{}/{}", base(), value.origin.operation_id)
}
pub async fn raw(
    ports: (MockWrite, MockRead, MockRecords),
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
    let response = web::measure_administrative_router(
        Arc::new(ports.0),
        Arc::new(ports.1),
        Arc::new(ports.2),
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
    write: MockWrite,
    read: MockRead,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    raw(
        (write, read, MockRecords::new()),
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
