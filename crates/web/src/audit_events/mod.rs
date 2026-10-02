//! Bounded consultation of existing audit events; chain verification is separate.
mod query;
mod response;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::audit_query::AuditEventWorkflow;
use axum::{
    extract::{RawQuery, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
struct AuditState {
    workflow: Arc<dyn AuditEventWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn AuditEventWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/audit/events", get(read))
        .with_state(AuditState { workflow, runtime })
}
async fn read(
    State(state): State<AuditState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let query = query::parse(raw.as_deref())?;
    let result = state
        .runtime
        .run(move || {
            let page = state.workflow.read(&token, query.clone())?;
            response::page(page, &query)
        })
        .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod runtime_tests;
