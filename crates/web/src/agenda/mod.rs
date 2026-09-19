//! Combined calendar reads with bounded, query-specific continuation.
mod cursor;
mod query;
mod response;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::agenda::AgendaWorkflow;
use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct AgendaState {
    workflow: Arc<dyn AgendaWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn AgendaWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/agenda", get(list))
        .with_state(AgendaState { workflow, runtime })
}

async fn list(
    State(state): State<AgendaState>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.list(&token, query))
        .await?;
    Ok(Json(response::page(page, query)?))
}
