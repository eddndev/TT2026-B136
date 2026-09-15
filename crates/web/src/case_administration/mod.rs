//! Staff case administration routes using the shared server work budget.

use application::cases::CaseWorkflow;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        DefaultBodyLimit, Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use domain::cases::CaseId;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use query::{HistoryQuery, IndexQuery};
use request::{Creation, Replacement, StatusChange};
use response::{Detail, HistoryPage, Page};

mod query;
mod request;
mod response;

#[derive(Clone)]
struct AdministrationState {
    workflow: Arc<dyn CaseWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn CaseWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/case-administrations", get(list))
        .route("/api/v1/penal-cases", post(create))
        .route("/api/v1/cases/:id/administration", get(detail).put(replace))
        .route("/api/v1/cases/:id/administration/history", get(history))
        .route(
            "/api/v1/cases/:id/administrative-status",
            put(change_status),
        )
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(AdministrationState { workflow, runtime })
}

async fn create(
    State(state): State<AdministrationState>,
    headers: HeaderMap,
    input: Result<Json<Creation>, JsonRejection>,
) -> Result<(StatusCode, Json<Detail>), ApiError> {
    let token = bearer_token(&headers)?;
    let Json(input) = input.map_err(json_error)?;
    let creation = input.validate()?;
    let row = state
        .runtime
        .run(move || state.workflow.register_penal(&token, creation))
        .await?;
    Ok((StatusCode::CREATED, Json(row.try_into()?)))
}

async fn replace(
    State(state): State<AdministrationState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Replacement>, JsonRejection>,
) -> Result<Json<Detail>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, values) = input.validate()?;
    let row = state
        .runtime
        .run(move || {
            state
                .workflow
                .replace_administration(&token, id, expected, values)
        })
        .await?;
    Ok(Json(row.try_into()?))
}

async fn change_status(
    State(state): State<AdministrationState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<StatusChange>, JsonRejection>,
) -> Result<Json<Detail>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, status) = input.values();
    let row = state
        .runtime
        .run(move || {
            state
                .workflow
                .change_administrative_status(&token, id, expected, status)
        })
        .await?;
    Ok(Json(row.try_into()?))
}

async fn list(
    State(state): State<AdministrationState>,
    headers: HeaderMap,
    input: Result<Query<IndexQuery>, QueryRejection>,
) -> Result<Json<Page>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(query_error)?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.list_administrations(&token, query))
        .await?;
    Ok(Json(page.try_into()?))
}

async fn detail(
    State(state): State<AdministrationState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Detail>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let row = state
        .runtime
        .run(move || state.workflow.get_administration(&token, id))
        .await?;
    Ok(Json(row.try_into()?))
}

async fn history(
    State(state): State<AdministrationState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<HistoryPage>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Query(input) = input.map_err(query_error)?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.administration_history(&token, id, query))
        .await?;
    Ok(Json(page.try_into()?))
}

fn parse_case(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}
fn json_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::payload_too_large(
            "case_administration_body_too_large",
            "case administration JSON exceeds 64 KiB",
        )
    } else {
        ApiError::invalid_body("invalid_json", "invalid case administration JSON object")
    }
}
fn query_error(_: QueryRejection) -> ApiError {
    ApiError::invalid_body(
        "invalid_query",
        "invalid case administration query parameters",
    )
}
