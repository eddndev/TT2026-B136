//! Authorized case participant routes using the shared server work budget.

use application::participants::{ParticipantId, ParticipantWorkflow};
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        DefaultBodyLimit, Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use domain::cases::CaseId;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use dto::{
    CreateRequest, HistoryResponse, PageResponse, ReplaceRequest, SnapshotResponse, StatusRequest,
};
use query::{HistoryQuery, ListQuery};

mod dto;
mod query;

#[derive(Clone)]
struct ParticipantState {
    workflow: Arc<dyn ParticipantWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn ParticipantWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/cases/:case/participants", get(list).post(create))
        .route(
            "/api/v1/cases/:case/participants/:id",
            get(detail).put(replace),
        )
        .route("/api/v1/cases/:case/participants/:id/history", get(history))
        .route(
            "/api/v1/cases/:case/participants/:id/directory-status",
            put(change_status),
        )
        .layer(DefaultBodyLimit::max(8 * 1024))
        .with_state(ParticipantState { workflow, runtime })
}

async fn create(
    State(state): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    input: Result<Json<CreateRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SnapshotResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let Json(input) = input.map_err(json_error)?;
    let row = state
        .runtime
        .run(move || {
            state.workflow.create(
                &token,
                case,
                &input.display_name,
                &input.procedural_role,
                input.organization.as_deref(),
                input.legal_status.as_deref(),
            )
        })
        .await?;
    Ok((StatusCode::CREATED, Json(row.try_into()?)))
}

async fn replace(
    State(state): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Json<ReplaceRequest>, JsonRejection>,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, values) = input.validate()?;
    let row = state
        .runtime
        .run(move || state.workflow.replace(&token, case, id, expected, values))
        .await?;
    Ok(Json(row.try_into()?))
}

async fn change_status(
    State(state): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Json<StatusRequest>, JsonRejection>,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, status) = input.validate()?;
    let row = state
        .runtime
        .run(move || {
            state
                .workflow
                .change_status(&token, case, id, expected, status)
        })
        .await?;
    Ok(Json(row.try_into()?))
}

async fn list(
    State(state): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    input: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<PageResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let Query(input) = input.map_err(query_error)?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.list(&token, case, query))
        .await?;
    Ok(Json(page.try_into()?))
}

async fn detail(
    State(state): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let row = state
        .runtime
        .run(move || state.workflow.get(&token, case, id))
        .await?;
    Ok(Json(row.try_into()?))
}

async fn history(
    State(state): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let Query(input) = input.map_err(query_error)?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.history(&token, case, id, query))
        .await?;
    Ok(Json(page.try_into()?))
}

fn parse_case(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}
fn parse_scope(case: &str, id: &str) -> Result<(CaseId, ParticipantId), ApiError> {
    Ok((
        parse_case(case)?,
        Uuid::parse_str(id)
            .map(ParticipantId::from_uuid)
            .map_err(|_| {
                ApiError::invalid_body("invalid_participant_id", "participant id must be a uuid")
            })?,
    ))
}
fn json_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::payload_too_large(
            "participant_body_too_large",
            "participant JSON exceeds 8 KiB",
        )
    } else {
        ApiError::invalid_body("invalid_json", "invalid participant JSON object")
    }
}
fn query_error(_: QueryRejection) -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid participant query parameters")
}
