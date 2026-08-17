//! Inbound HTTP adapter.
//!
//! The router is deliberately small until the application services and
//! persistence ports are wired. The health contract gives deployment and
//! integration tests a stable entry point without coupling the web adapter to
//! an external timestamp provider.

use axum::{routing::get, Router};

/// Builds the inbound HTTP router.
pub fn router() -> Router {
    Router::new().route("/healthz", get(health))
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
