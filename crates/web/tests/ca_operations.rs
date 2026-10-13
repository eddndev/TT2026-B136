#[allow(dead_code)]
mod ca_access_support;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_login_cases/mod.rs"]
mod identity_fixture;

use std::sync::{Arc, Mutex};

use application::ca_access::{CaOperatorAccess, OperatorIdentity};
use application::ca_operations::{CaCommand, CaExecutor, CaOperations};
use application::ApplicationError;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use ca_access_support::{send, Harness};
use serde_json::{json, Value};
use web::HttpLimits;

#[derive(Default)]
struct Executor(Mutex<Vec<OperatorIdentity>>);
impl CaExecutor for Executor {
    fn execute(&self, actor: &OperatorIdentity, _: CaCommand) -> Result<Value, ApplicationError> {
        self.0.lock().unwrap().push(actor.clone());
        Ok(json!({"accepted":true}))
    }
}

#[tokio::test]
async fn ca_commands_bind_the_authenticated_operator_and_reject_forged_actor_or_office_tokens() {
    let harness = Harness::new();
    let executor = Arc::new(Executor::default());
    let access = Arc::new(CaOperatorAccess::new(
        harness.identity.clone(),
        harness.operator.clone(),
    ));
    let operations = Arc::new(CaOperations::new(access, executor.clone()));
    let router = web::ca_operations::ca_operations_router(operations, HttpLimits::default());
    let token = harness.raw_session();
    for (bearer, payload, expected) in [
        (
            "office-token",
            json!({"action":"status"}),
            StatusCode::UNAUTHORIZED,
        ),
        (
            token.as_str(),
            json!({"action":"status","operator_id":"forged"}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            token.as_str(),
            json!({"action":"shell","command":"echo nope"}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (token.as_str(), json!({"action":"status"}), StatusCode::OK),
    ] {
        let request = Request::post("/api/v1/ca/operations")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {bearer}"))
            .body(Body::from(payload.to_string()))
            .unwrap();
        let response = send(&router, request).await;
        assert_eq!(response.status, expected);
        assert_eq!(response.headers["cache-control"], "no-store");
    }
    assert_eq!(*executor.0.lock().unwrap(), vec![harness.operator.clone()]);
    harness.identity.logout(&token).unwrap();
    let request = Request::post("/api/v1/ca/operations")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(json!({"action":"status"}).to_string()))
        .unwrap();
    assert_eq!(
        send(&router, request).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(executor.0.lock().unwrap().len(), 1);
}
