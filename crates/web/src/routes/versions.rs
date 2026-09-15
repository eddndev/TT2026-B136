//! Exact document snapshots and compare-and-append HTTP contracts.

use application::documents::{DocumentVersionRef, VersionQuery};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use domain::{cases::CaseId, crypto::DocumentVersion};
use serde::{Deserialize, Serialize};

use super::documents::{evidence_response, parse_case_id, parse_id, required_header};
use super::AppState;
use crate::{
    dto::{DocumentResponse, VerificationResponse},
    error::ApiError,
    request::bearer_token,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppendVersion {
    expected_version: u32,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ListVersions {
    limit: u32,
    before_version: Option<u32>,
}

impl Default for ListVersions {
    fn default() -> Self {
        Self {
            limit: 50,
            before_version: None,
        }
    }
}

#[derive(Serialize)]
pub(super) struct VersionPageResponse {
    versions: Vec<DocumentResponse>,
    has_more: bool,
    next_before_version: Option<u32>,
    first_available_version: u32,
}

#[derive(Serialize)]
pub(super) struct VersionVerificationResponse {
    case_id: String,
    id: String,
    version: u32,
    #[serde(flatten)]
    report: VerificationResponse,
}

pub(super) async fn append_version(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
    Query(input): Query<AppendVersion>,
    body: Bytes,
) -> Result<(StatusCode, Json<DocumentResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let expected = DocumentVersion::new(input.expected_version).map_err(|_| {
        application::ApplicationError::InvalidInput("expected version must be positive".into())
    })?;
    let name = required_header(&headers, "x-document-name")?.to_string();
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.append(&token, case_id, id, expected, &name, &body))
        .await?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}

pub(super) async fn list_versions(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
    Query(input): Query<ListVersions>,
) -> Result<Json<VersionPageResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let query = VersionQuery::new(input.limit, input.before_version)?;
    let workflow = state.workflow.clone();
    let page = state
        .runtime
        .run(move || workflow.history(&token, case_id, id, query))
        .await?;
    Ok(Json(VersionPageResponse {
        versions: page.versions.into_iter().map(Into::into).collect(),
        has_more: page.has_more,
        next_before_version: page.next_before_version.map(|version| version.get()),
        first_available_version: page.first_available_version.get(),
    }))
}

pub(super) async fn get_version(
    State(state): State<AppState>,
    Path(path): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case_id, reference) = parse_reference(path)?;
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.get_version(&token, case_id, reference))
        .await?;
    Ok(Json(summary.into()))
}

pub(super) async fn seal_version(
    State(state): State<AppState>,
    Path(path): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case_id, reference) = parse_reference(path)?;
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.seal_version(&token, case_id, reference))
        .await?;
    Ok(Json(summary.into()))
}

pub(super) async fn verify_version(
    State(state): State<AppState>,
    Path(path): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<VersionVerificationResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case_id, reference) = parse_reference(path)?;
    let workflow = state.workflow.clone();
    let report = state
        .runtime
        .run(move || workflow.verify_version(&token, case_id, reference))
        .await?;
    Ok(Json(VersionVerificationResponse {
        case_id: case_id.to_string(),
        id: reference.id.to_string(),
        version: reference.version.get(),
        report: report.into(),
    }))
}

pub(super) async fn export_version(
    State(state): State<AppState>,
    Path(path): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer_token(&headers)?;
    let (case_id, reference) = parse_reference(path)?;
    let workflow = state.workflow.clone();
    let export = state
        .runtime
        .run(move || workflow.export_version(&token, case_id, reference))
        .await?;
    let mut response = evidence_response(export)?;
    for (name, value) in [
        ("x-document-id", reference.id.to_string()),
        ("x-document-version", reference.version.get().to_string()),
    ] {
        response.headers_mut().insert(
            name,
            HeaderValue::from_str(&value).map_err(|_| ApiError::invalid_response_header())?,
        );
    }
    Ok(response)
}

fn parse_reference(
    (case_id, id, version): (String, String, String),
) -> Result<(CaseId, DocumentVersionRef), ApiError> {
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let version = version
        .parse::<u32>()
        .ok()
        .and_then(|value| DocumentVersion::new(value).ok())
        .ok_or_else(ApiError::invalid_document_version)?;
    Ok((case_id, DocumentVersionRef { id, version }))
}
