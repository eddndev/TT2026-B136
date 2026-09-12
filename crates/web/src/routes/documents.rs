//! Document and audit endpoints preserve the local workflow contract.

use super::AppState;
use crate::dto::{AuditResponse, DocumentResponse, VerificationResponse};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::Response,
    Json,
};
use domain::{crypto::DocumentId, identity::Permission};
use uuid::Uuid;

const DOCUMENT_NAME_HEADER: &str = "x-document-name";

pub(super) async fn upload_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<DocumentResponse>), ApiError> {
    let actor = authorize(&state, &headers, Permission::CreateDocument).await?;
    let name = required_header(&headers, DOCUMENT_NAME_HEADER)?.to_string();
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.upload(&actor.email, &name, &body))
        .await?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}

pub(super) async fn seal_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let actor = authorize(&state, &headers, Permission::SealDocument).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.seal(&actor.email, id))
        .await?;
    Ok(Json(summary.into()))
}

pub(super) async fn verify_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<VerificationResponse>, ApiError> {
    let actor = authorize(&state, &headers, Permission::VerifyDocument).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let report = state
        .runtime
        .run(move || workflow.verify(&actor.email, id))
        .await?;
    Ok(Json(report.into()))
}

pub(super) async fn export_evidence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authorize(&state, &headers, Permission::ExportEvidence).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let export = state
        .runtime
        .run(move || workflow.export_evidence(&actor.email, id))
        .await?;
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

pub(super) async fn verify_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuditResponse>, ApiError> {
    authorize(&state, &headers, Permission::VerifyAudit).await?;
    let workflow = state.workflow.clone();
    Ok(Json(
        state
            .runtime
            .run(move || workflow.verify_audit())
            .await?
            .into(),
    ))
}

async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<application::identity::Principal, ApiError> {
    let token = bearer_token(headers)?;
    let identity = state.identity.clone();
    state
        .runtime
        .run(move || identity.authorize(&token, permission))
        .await
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
