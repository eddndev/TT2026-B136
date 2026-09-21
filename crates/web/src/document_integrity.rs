//! Owner-only projections of durable content-validation observations.

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::document_integrity::{
    DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityQuery,
    DocumentIntegrityWorkflow,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;

#[derive(Clone)]
struct IntegrityState {
    workflow: Arc<dyn DocumentIntegrityWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn DocumentIntegrityWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/document-integrity-incidents", get(list))
        .route("/api/v1/document-integrity-incidents/:id", get(detail))
        .with_state(IntegrityState { workflow, runtime })
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Input {
    limit: u32,
    after_id: Option<String>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            limit: 50,
            after_id: None,
        }
    }
}

#[derive(Serialize)]
struct IncidentResponse {
    id: String,
    observation_id: String,
    case_id: String,
    document_id: String,
    document_version: u32,
    requester_id: String,
    failure: &'static str,
    detected_at: String,
    recorded_at: String,
    expected_digest: String,
    observed_snapshot_digest: String,
}

impl TryFrom<DocumentIntegrityIncident> for IncidentResponse {
    type Error = ApiError;
    fn try_from(record: DocumentIntegrityIncident) -> Result<Self, Self::Error> {
        Ok(Self {
            id: record.id.to_string(),
            observation_id: record.observation_id.to_string(),
            case_id: record.case_id.to_string(),
            document_id: record.reference.id.to_string(),
            document_version: record.reference.version.get(),
            requester_id: record.requester.to_string(),
            failure: record.failure.as_str(),
            detected_at: record
                .detected_at
                .to_offset(time::UtcOffset::UTC)
                .format(&Rfc3339)
                .map_err(|_| ApiError::internal())?,
            recorded_at: record
                .recorded_at
                .to_offset(time::UtcOffset::UTC)
                .format(&Rfc3339)
                .map_err(|_| ApiError::internal())?,
            expected_digest: record.expected_digest.to_hex(),
            observed_snapshot_digest: record.observed_snapshot_digest.to_hex(),
        })
    }
}

#[derive(Serialize)]
struct PageResponse {
    incidents: Vec<IncidentResponse>,
    has_more: bool,
    next_after_id: Option<String>,
}

fn incident_id(value: &str) -> Result<DocumentIntegrityIncidentId, ApiError> {
    value
        .parse()
        .map(DocumentIntegrityIncidentId::from_uuid)
        .map_err(|_| ApiError::invalid_body("invalid_incident_id", "incident id must be a uuid"))
}

async fn list(
    State(state): State<IntegrityState>,
    headers: HeaderMap,
    input: Result<Query<Input>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PageResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(|_| {
        ApiError::invalid_body(
            "invalid_incident_query",
            "invalid incident pagination parameters",
        )
    })?;
    if !(1..=100).contains(&input.limit) {
        return Err(ApiError::invalid_body(
            "invalid_incident_query",
            "incident limit must be between 1 and 100",
        ));
    }
    let after = input.after_id.as_deref().map(incident_id).transpose()?;
    let query = DocumentIntegrityQuery::new(input.limit, after)?;
    let workflow = state.workflow;
    let page = state
        .runtime
        .run(move || workflow.list(&token, query))
        .await?;
    Ok(Json(PageResponse {
        incidents: page
            .incidents
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?,
        has_more: page.has_more,
        next_after_id: page.next_after_id.map(|id| id.to_string()),
    }))
}

async fn detail(
    State(state): State<IntegrityState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<IncidentResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = incident_id(&id)?;
    let workflow = state.workflow;
    let incident = state.runtime.run(move || workflow.get(&token, id)).await?;
    if incident.id != id {
        return Err(ApiError::internal());
    }
    Ok(Json(incident.try_into()?))
}
