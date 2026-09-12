//! Inbound HTTP adapter.
//!
//! Routes adapt HTTP to injected identity, document, and case workflows.
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
mod request;
mod routes;
mod runtime;
pub use runtime::HttpLimits;
use runtime::{protect, HttpRuntime};

/// Builds the inbound HTTP router.
pub fn router() -> Router {
    Router::new().route("/healthz", get(health))
}

/// Builds the versioned application API over an injected workflow.
pub fn application_router(
    workflow: Arc<dyn DocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(routes::router(workflow, identity, runtime.clone()), runtime)
        .route("/healthz", get(health))
}

/// Builds case routes whose workflow authenticates and authorizes each request.
pub fn case_router(workflow: Arc<dyn CaseWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(cases::router(workflow, runtime.clone()), runtime)
}

/// Builds all API routes with one shared admission and blocking-work budget.
pub fn api_router(
    documents: Arc<dyn DocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    cases: Arc<dyn CaseWorkflow>,
    limits: HttpLimits,
) -> Router {
    let runtime = HttpRuntime::new(limits);
    let routes = routes::router(documents, identity, runtime.clone())
        .merge(cases::router(cases, runtime.clone()));
    protect(routes, runtime).route("/healthz", get(health))
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
