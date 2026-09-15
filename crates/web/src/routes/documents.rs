//! Case-scoped document and audit endpoints delegate authorization to use cases.

use super::AppState;
use crate::dto::{AuditResponse, DocumentOverviewResponse, DocumentResponse, VerificationResponse};
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
use domain::{cases::CaseId, crypto::DocumentId};
use uuid::Uuid;

const DOCUMENT_NAME_HEADER: &str = "x-document-name";

pub(super) async fn upload_document(
    State(state): State<AppState>,
    Path(case_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<DocumentOverviewResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let name = required_header(&headers, DOCUMENT_NAME_HEADER)?.to_string();
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.upload(&token, case_id, &name, &body))
        .await?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}

pub(super) async fn seal_document(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.seal(&token, case_id, id))
        .await?;
    Ok(Json(summary.into()))
}

pub(super) async fn verify_document(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<VerificationResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let report = state
        .runtime
        .run(move || workflow.verify(&token, case_id, id))
        .await?;
    Ok(Json(report.into()))
}

pub(super) async fn export_evidence(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let export = state
        .runtime
        .run(move || workflow.export_evidence(&token, case_id, id))
        .await?;
    evidence_response(export)
}

pub(super) fn evidence_response(
    export: application::documents::EvidenceExport,
) -> Result<Response, ApiError> {
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
    let token = bearer_token(&headers)?;
    let workflow = state.workflow.clone();
    Ok(Json(
        state
            .runtime
            .run(move || workflow.verify_audit(&token))
            .await?
            .into(),
    ))
}

pub(super) fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, ApiError> {
    let mut values = headers.get_all(name).iter();
    let value = values.next().and_then(|value| value.to_str().ok());
    if values.next().is_some() {
        return Err(ApiError::invalid_header(name));
    }
    let value = value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::invalid_header(name))?;
    Ok(value)
}

pub(super) fn parse_id(value: &str) -> Result<DocumentId, ApiError> {
    Uuid::parse_str(value)
        .map(DocumentId::from_uuid)
        .map_err(|_| ApiError::invalid_document_id())
}

pub(super) fn parse_case_id(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}
