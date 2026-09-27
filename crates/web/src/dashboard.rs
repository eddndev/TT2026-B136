//! Authorized operational metrics from a single audited snapshot.
use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::{
    dashboard::{DashboardSnapshot, DashboardWorkflow},
    ApplicationError,
};
use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;

#[derive(Clone)]
struct DashboardState {
    workflow: Arc<dyn DashboardWorkflow>,
    runtime: HttpRuntime,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoFilters {}

pub(crate) fn router(workflow: Arc<dyn DashboardWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/dashboard", get(read))
        .with_state(DashboardState { workflow, runtime })
}
async fn read(
    State(state): State<DashboardState>,
    headers: HeaderMap,
    query: Result<Query<NoFilters>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query.map_err(|_| {
        ApiError::invalid_body("invalid_dashboard_query", "dashboard accepts no filters")
    })?;
    let snapshot = state
        .runtime
        .run(move || state.workflow.read(&token))
        .await?;
    Ok(Json(response(snapshot)?))
}
fn response(snapshot: DashboardSnapshot) -> Result<Value, ApplicationError> {
    let checked_at = snapshot
        .checked_at
        .format(&Rfc3339)
        .map_err(|_| ApplicationError::Port("invalid dashboard timestamp".into()))?;
    Ok(json!({
        "checked_at": checked_at, "scope": snapshot.scope.as_str(),
        "active_cases": snapshot.active_cases, "pending_contracts": snapshot.pending_contracts,
        "deadlines_overdue": snapshot.deadlines_overdue,
        "deadlines_due_48h": snapshot.deadlines_due_48h,
        "deadlines_due_7d": snapshot.deadlines_due_7d,
        "deadlines_unresolved": snapshot.deadlines_unresolved,
        "workload": snapshot.workload.into_iter().map(|entry| json!({
            "user_id": entry.user_id.to_string(), "email": entry.email, "active_cases": entry.active_cases
        })).collect::<Vec<_>>()
    }))
}
