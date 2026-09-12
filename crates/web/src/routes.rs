//! Versioned identity and document route composition.

use crate::runtime::HttpRuntime;
use application::{documents::CaseDocumentWorkflow, identity::IdentityWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

mod documents;
mod identity;
use documents::{export_evidence, seal_document, upload_document, verify_audit, verify_document};
use identity::{
    bootstrap_owner, complete_recovery, complete_totp, create_user, current_user, logout,
    start_login,
};

const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    workflow: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    runtime: HttpRuntime,
}

pub fn router(
    workflow: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    runtime: HttpRuntime,
) -> Router {
    let identity_routes = Router::new()
        .route("/api/v1/auth/bootstrap", post(bootstrap_owner))
        .route("/api/v1/auth/login", post(start_login))
        .route("/api/v1/auth/mfa/totp", post(complete_totp))
        .route("/api/v1/auth/mfa/recovery", post(complete_recovery))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(current_user))
        .route("/api/v1/users", post(create_user))
        .layer(DefaultBodyLimit::max(16 * 1024));
    Router::new()
        .merge(identity_routes)
        .route("/api/v1/cases/:case_id/documents", post(upload_document))
        .route(
            "/api/v1/cases/:case_id/documents/:id/seal",
            post(seal_document),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/verify",
            post(verify_document),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/evidence",
            get(export_evidence),
        )
        .route("/api/v1/audit/verify", get(verify_audit))
        .layer(DefaultBodyLimit::max(MAX_DOCUMENT_BYTES))
        .with_state(AppState {
            workflow,
            identity,
            runtime,
        })
}
