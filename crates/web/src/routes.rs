//! Versioned identity and document route composition.

use crate::runtime::HttpRuntime;
use application::{documents::CaseDocumentWorkflow, identity::IdentityWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

mod classified_upload;
mod documents;
mod identity;
mod metadata;
mod queries;
mod versions;
use documents::{export_evidence, seal_document, upload_document, verify_audit, verify_document};
use identity::{
    bootstrap_owner, complete_recovery, complete_totp, create_user, current_user, logout,
    start_login,
};
use queries::{get_document, list_documents};
use versions::{
    append_version, export_version, get_version, list_versions, seal_version, verify_version,
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
        .merge(
            Router::new()
                .route(
                    "/api/v1/cases/:case_id/documents/with-metadata",
                    post(classified_upload::upload_with_metadata),
                )
                .layer(DefaultBodyLimit::max(
                    classified_upload::MAX_CLASSIFIED_UPLOAD_BYTES,
                )),
        )
        .merge(
            Router::new()
                .route(
                    "/api/v1/cases/:case_id/documents/:id/metadata",
                    get(metadata::get_metadata).put(metadata::replace_metadata),
                )
                .route(
                    "/api/v1/cases/:case_id/documents/:id/metadata/history",
                    get(metadata::metadata_history),
                )
                .layer(DefaultBodyLimit::max(metadata::MAX_METADATA_BYTES)),
        )
        .route(
            "/api/v1/cases/:case_id/documents",
            get(list_documents).post(upload_document),
        )
        .route("/api/v1/cases/:case_id/documents/:id", get(get_document))
        .route(
            "/api/v1/cases/:case_id/documents/:id/versions",
            get(list_versions).post(append_version),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/versions/:version",
            get(get_version),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/versions/:version/seal",
            post(seal_version),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/versions/:version/verify",
            post(verify_version),
        )
        .route(
            "/api/v1/cases/:case_id/documents/:id/versions/:version/evidence",
            get(export_version),
        )
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
