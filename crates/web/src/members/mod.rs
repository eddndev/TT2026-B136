//! Owner directory, current access and authorized case assignment selectors.

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::members::MemberWorkflow;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        DefaultBodyLimit, Path, Query, State,
    },
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use domain::{cases::CaseId, identity::UserId};
use std::sync::Arc;
mod input;
mod output;
use input::{invalid, AccessInput, AssignmentInput, DirectoryInput, NoQuery};
use output::{AssignmentPageResponse, DirectoryResponse, UserResponse};

#[derive(Clone)]
struct MemberState {
    workflow: Arc<dyn MemberWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn MemberWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/users", get(list))
        .route("/api/v1/users/:id", get(detail))
        .route("/api/v1/users/:id/access", put(change))
        .route("/api/v1/cases/:case_id/members", get(assignments))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(MemberState { workflow, runtime })
}
fn id(value: &str) -> Result<UserId, ApiError> {
    value.parse().map(UserId::from_uuid).map_err(|_| invalid())
}
async fn list(
    State(state): State<MemberState>,
    headers: HeaderMap,
    input: Result<Query<DirectoryInput>, QueryRejection>,
) -> Result<Json<DirectoryResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let query = input.map_err(|_| invalid())?.0.query()?;
    let workflow = state.workflow;
    let page = state
        .runtime
        .run(move || workflow.list(&token, query))
        .await?;
    Ok(Json(page.try_into()?))
}
async fn detail(
    State(state): State<MemberState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    query: Result<Query<NoQuery>, QueryRejection>,
) -> Result<Json<UserResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    query.map_err(|_| invalid())?;
    let id = id(&raw)?;
    let workflow = state.workflow;
    let user = state.runtime.run(move || workflow.get(&token, id)).await?;
    Ok(Json(user.try_into()?))
}
async fn change(
    State(state): State<MemberState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    query: Result<Query<NoQuery>, QueryRejection>,
    input: Result<Json<AccessInput>, JsonRejection>,
) -> Result<Json<UserResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    query.map_err(|_| invalid())?;
    let id = id(&raw)?;
    let change = input.map_err(|_| invalid())?.0.change()?;
    let workflow = state.workflow;
    let user = state
        .runtime
        .run(move || workflow.change_access(&token, id, change))
        .await?;
    Ok(Json(user.try_into()?))
}
async fn assignments(
    State(state): State<MemberState>,
    headers: HeaderMap,
    Path(raw): Path<String>,
    input: Result<Query<AssignmentInput>, QueryRejection>,
) -> Result<Json<AssignmentPageResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = raw.parse().map(CaseId::from_uuid).map_err(|_| invalid())?;
    let query = input.map_err(|_| invalid())?.0.query(case_id)?;
    let workflow = state.workflow;
    let page = state
        .runtime
        .run(move || workflow.list_case_members(&token, case_id, query))
        .await?;
    Ok(Json(page.try_into()?))
}
