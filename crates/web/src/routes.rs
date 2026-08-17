//! Axum routes that adapt HTTP requests to the document workflow port.

use std::sync::Arc;

use application::documents::DocumentWorkflow;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::crypto::DocumentId;
use uuid::Uuid;

use crate::dto::{AuditResponse, DocumentResponse, VerificationResponse};
use crate::error::ApiError;

const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const ACTOR_HEADER: &str = "x-actor";
const DOCUMENT_NAME_HEADER: &str = "x-document-name";

#[derive(Clone)]
struct AppState {
    workflow: Arc<dyn DocumentWorkflow>,
}

pub fn router(workflow: Arc<dyn DocumentWorkflow>) -> Router {
    Router::new()
        .route("/api/v1/documents", post(upload_document))
        .route("/api/v1/documents/:id/seal", post(seal_document))
        .route("/api/v1/documents/:id/verify", post(verify_document))
        .route("/api/v1/documents/:id/evidence", get(export_evidence))
        .route("/api/v1/audit/verify", get(verify_audit))
        .layer(DefaultBodyLimit::max(MAX_DOCUMENT_BYTES))
        .with_state(AppState { workflow })
}

async fn upload_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<DocumentResponse>), ApiError> {
    let actor = required_header(&headers, ACTOR_HEADER)?;
    let name = required_header(&headers, DOCUMENT_NAME_HEADER)?;
    let summary = state.workflow.upload(actor, name, &body)?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}

async fn seal_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let actor = required_header(&headers, ACTOR_HEADER)?;
    let summary = state.workflow.seal(actor, parse_id(&id)?)?;
    Ok(Json(summary.into()))
}

async fn verify_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<VerificationResponse>, ApiError> {
    let actor = required_header(&headers, ACTOR_HEADER)?;
    let report = state.workflow.verify(actor, parse_id(&id)?)?;
    Ok(Json(report.into()))
}

async fn export_evidence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = required_header(&headers, ACTOR_HEADER)?;
    let export = state.workflow.export_evidence(actor, parse_id(&id)?)?;
    let disposition = format!("attachment; filename=\"{}\"", export.file_name);
    let mut response = Response::new(Body::from(export.archive));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).map_err(|_| ApiError::invalid_response_header())?,
    );
    response.headers_mut().insert(
        "x-document-digest",
        HeaderValue::from_str(&export.document_digest_hex)
            .map_err(|_| ApiError::invalid_response_header())?,
    );
    Ok(response)
}

async fn verify_audit(State(state): State<AppState>) -> Result<Json<AuditResponse>, ApiError> {
    Ok(Json(state.workflow.verify_audit()?.into()))
}

fn required_header<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, ApiError> {
    let value = headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::invalid_header(name))?;
    Ok(value)
}

fn parse_id(value: &str) -> Result<DocumentId, ApiError> {
    Uuid::parse_str(value)
        .map(DocumentId::from_uuid)
        .map_err(|_| ApiError::invalid_document_id())
}
