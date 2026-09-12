//! Inbound HTTP adapter.
//!
//! Health and document routes adapt HTTP to an injected application workflow.
//! Cryptography, persistence, and timestamps remain behind application ports,
//! so this adapter does not depend on an external timestamp provider.

use std::sync::Arc;

use application::cases::CaseWorkflow;
use application::documents::DocumentWorkflow;
use application::identity::IdentityWorkflow;
use axum::{routing::get, Router};

mod cases;
mod dto;
mod error;
mod routes;

/// Builds the inbound HTTP router.
pub fn router() -> Router {
    Router::new().route("/healthz", get(health))
}

/// Builds the versioned application API over an injected workflow.
pub fn application_router(
    workflow: Arc<dyn DocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
) -> Router {
    routes::router(workflow, identity).route("/healthz", get(health))
}

/// Builds case routes whose workflow authenticates and authorizes each request.
pub fn case_router(workflow: Arc<dyn CaseWorkflow>) -> Router {
    cases::router(workflow)
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::router;
    use axum::{body::to_bytes, body::Body, http::Request, http::StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_reports_ready() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }
}
