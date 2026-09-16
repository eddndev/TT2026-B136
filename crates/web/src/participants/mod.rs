//! Authorized case participant routes using the shared server work budget.

use application::participants::{ParticipantId, ParticipantRevision, ParticipantWorkflow};
use axum::{
    extract::{rejection::QueryRejection, DefaultBodyLimit, Path, Query, Request, State},
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
            "/api/v1/cases/:case/participants/:id/revisions/:revision",
            get(exact_revision),
        )
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
    request: Request,
) -> Result<(StatusCode, Json<SnapshotResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let input: CreateRequest = read_json(request).await?;
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
    request: Request,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let input: ReplaceRequest = read_json(request).await?;
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
    request: Request,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let input: StatusRequest = read_json(request).await?;
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
async fn read_json<T: serde::de::DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read(
        request,
        8 * 1024,
        "participant_body_too_large",
        "participant JSON exceeds 8 KiB",
    )
    .await
}
fn query_error(_: QueryRejection) -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid participant query parameters")
}

async fn exact_revision(
    State(state): State<ParticipantState>,
    Path((case, id, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = parse_scope(&case, &id)?;
    let revision = revision
        .parse::<u32>()
        .ok()
        .filter(|v| *v > 0)
        .filter(|_| revision.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            ApiError::invalid_body("invalid_revision", "revision must be a positive integer")
        })?;
    let revision =
        ParticipantRevision::new(revision).map_err(application::ApplicationError::from)?;
    let row = state
        .runtime
        .run(move || state.workflow.get_revision(&token, case, id, revision))
        .await?;
    Ok(Json(row.try_into()?))
}
