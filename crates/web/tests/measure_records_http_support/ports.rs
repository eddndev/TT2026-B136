use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use mockall::mock;
use std::sync::Arc;
use tower::ServiceExt;

mock! {
    pub Administrative {}
    impl MeasureAdministrativeWorkflow for Administrative {
        fn prepare(&self, token:&str, case:CaseId, command:MeasureAdministrativeCommand)->Result<MeasureAdministrativeReview,ApplicationError>;
        fn submit(&self, token:&str, case:CaseId, command:MeasureAdministrativeCommand, confirmation:MeasureAdministrativeConfirmation)->Result<MeasureAdministrativeStoredOperation,ApplicationError>;
    }
}
mock! {
    pub AdministrativeReads {}
    impl MeasureAdministrativeReadWorkflow for AdministrativeReads {
        fn list(&self, token:&str, case:CaseId, query:MeasureAdministrativeReadQuery)->Result<MeasureAdministrativePage,ApplicationError>;
        fn get_operation(&self, token:&str, case:CaseId, operation:MeasureCorrectionOperationId)->Result<MeasureAdministrativeStoredOperation,ApplicationError>;
    }
}
mock! {
    pub Records {}
    impl MeasureRecordReadWorkflow for Records {
        fn list(&self, token:&str, case:CaseId, query:MeasureRecordReadQuery)->Result<MeasureRecordPage,ApplicationError>;
        fn get(&self, token:&str, case:CaseId, id:MeasureId)->Result<MeasureRecordDetail,ApplicationError>;
        fn exact(&self, token:&str, case:CaseId, reference:PrecautionaryMeasureRef)->Result<MeasureRecordDetail,ApplicationError>;
    }
}
pub async fn raw(records: MockRecords, path: &str, token: Option<&str>) -> (u16, Value) {
    let mut request = Request::builder().method("GET").uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = web::measure_administrative_router(
        Arc::new(MockAdministrative::new()),
        Arc::new(MockAdministrativeReads::new()),
        Arc::new(records),
        Arc::new(Hasher),
    )
    .oneshot(request.body(Body::empty()).unwrap())
    .await
    .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn request(records: MockRecords, path: &str) -> (u16, Value) {
    raw(records, path, Some("staff-token")).await
}
