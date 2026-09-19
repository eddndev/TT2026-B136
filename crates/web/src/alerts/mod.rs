//! Personal alert transport; authorization and delivery remain behind ports.
mod input;
mod response;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::alerts::{AlertReadCommand, AlertWorkflow};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, Request, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct AlertState {
    workflow: Arc<dyn AlertWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn AlertWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/alert-preferences",
            get(preferences).put(save_preferences),
        )
        .route("/api/v1/alerts", get(list))
        .route("/api/v1/alerts/:id", get(detail))
        .route("/api/v1/alerts/:id/read", post(mark_read))
        .with_state(AlertState { workflow, runtime })
}

async fn preferences(
    State(state): State<AlertState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let preferences = state
        .runtime
        .run(move || state.workflow.preferences(&token))
        .await?;
    Ok(Json(response::preferences(preferences)?))
}
async fn save_preferences(
    State(state): State<AlertState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(request.headers())?;
    let command = input::body::<input::Preferences>(request)
        .await?
        .validate()?;
    let expected = command.clone();
    let preferences = state
        .runtime
        .run(move || state.workflow.save_preferences(&token, command))
        .await?;
    preferences
        .validate_command(preferences.user_id, &expected)
        .map_err(|_| ApiError::internal())?;
    Ok(Json(response::preferences(preferences)?))
}
async fn list(
    State(state): State<AlertState>,
    headers: HeaderMap,
    input: Result<Query<input::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(|_| input::malformed())?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.list(&token, query))
        .await?;
    Ok(Json(response::page(page, query)?))
}
async fn detail(
    State(state): State<AlertState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = input::id(&id)?;
    let detail = state
        .runtime
        .run(move || state.workflow.get(&token, id))
        .await?;
    if detail.alert.id != id {
        return Err(ApiError::internal());
    }
    Ok(Json(response::detail(detail)?))
}
async fn mark_read(
    State(state): State<AlertState>,
    Path(id): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(request.headers())?;
    let alert_id = input::id(&id)?;
    let operation_id = input::body::<input::Read>(request).await?.validate()?;
    let command = AlertReadCommand {
        operation_id,
        alert_id,
    };
    let receipt = state
        .runtime
        .run(move || state.workflow.mark_read(&token, command))
        .await?;
    if receipt.operation_id != operation_id
        || receipt.alert.id != alert_id
        || receipt.alert.read_at.is_none()
    {
        return Err(ApiError::internal());
    }
    Ok(Json(response::receipt(receipt)?))
}
