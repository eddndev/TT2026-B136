//! Procedural stage declarations over authenticated application workflows.

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::case_stages::CaseStageWorkflow;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        DefaultBodyLimit, Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use domain::cases::CaseId;
use request::{Adoption, HistoryQuery, Transition};
use response::{Detail, HistoryPage};
use std::sync::Arc;
use uuid::Uuid;

mod date;
mod request;
pub(crate) mod response;
mod values;

#[derive(Clone)]
struct StageState {
    workflow: Arc<dyn CaseStageWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn CaseStageWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/cases/:id/stage", get(detail))
        .route("/api/v1/cases/:id/stage/history", get(history))
        .route("/api/v1/cases/:id/stage/adoption", post(adopt))
        .route("/api/v1/cases/:id/stage/transitions", post(transition))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .with_state(StageState { workflow, runtime })
}
async fn detail(
    State(state): State<StageState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Detail>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let row = state
        .runtime
        .run(move || state.workflow.get(&token, id))
        .await?;
    Ok(Json(Detail::from_row(row, id)?))
}
async fn history(
    State(state): State<StageState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Query<HistoryQuery>, QueryRejection>,
) -> Result<Json<HistoryPage>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Query(input) = input.map_err(|_| {
        ApiError::invalid_body("invalid_query", "invalid stage history query parameters")
    })?;
    let query = input.validate()?;
    let row = state
        .runtime
        .run(move || state.workflow.history(&token, id, query))
        .await?;
    Ok(Json(HistoryPage::from_page(row, id)?))
}
async fn adopt(
    State(state): State<StageState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Adoption>, JsonRejection>,
) -> Result<(StatusCode, Json<Detail>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, values) = input.validate()?;
    let row = state
        .runtime
        .run(move || state.workflow.adopt(&token, id, expected, values))
        .await?;
    Ok((StatusCode::CREATED, Json(Detail::from_row(row, id)?)))
}
async fn transition(
    State(state): State<StageState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Transition>, JsonRejection>,
) -> Result<(StatusCode, Json<Detail>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case(&id)?;
    let Json(input) = input.map_err(json_error)?;
    let (expected, values) = input.validate()?;
    let row = state
        .runtime
        .run(move || state.workflow.transition(&token, id, expected, values))
        .await?;
    Ok((StatusCode::CREATED, Json(Detail::from_row(row, id)?)))
}
fn parse_case(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}
fn json_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::payload_too_large(
            "case_stage_body_too_large",
            "case stage JSON exceeds 32 KiB",
        )
    } else {
        ApiError::invalid_body("invalid_json", "invalid case stage JSON object")
    }
}
