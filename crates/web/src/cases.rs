//! HTTP adaptation for authenticated case and membership workflows.

use std::sync::Arc;

use application::cases::{CaseRecord, CaseWorkflow};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router};
use domain::cases::CaseId;
use domain::identity::UserId;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::request::bearer_token;
use crate::runtime::HttpRuntime;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateCase {
    title: String,
    reference: String,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ListCases {
    limit: u32,
    offset: u32,
}

impl Default for ListCases {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}

#[derive(Clone)]
struct CaseState {
    workflow: Arc<dyn CaseWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn CaseWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/cases", get(list).post(create))
        .route("/api/v1/cases/:id", get(detail))
        .route(
            "/api/v1/cases/:id/members/:user_id",
            put(assign).delete(remove),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(CaseState { workflow, runtime })
}

async fn create(
    State(CaseState { workflow, runtime }): State<CaseState>,
    headers: HeaderMap,
    Json(input): Json<CreateCase>,
) -> Result<(StatusCode, Json<CaseRecord>), ApiError> {
    let token = bearer_token(&headers)?;
    let record = runtime
        .run(move || workflow.create(&token, &input.title, &input.reference))
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn list(
    State(CaseState { workflow, runtime }): State<CaseState>,
    headers: HeaderMap,
    Query(query): Query<ListCases>,
) -> Result<Json<Vec<CaseRecord>>, ApiError> {
    let token = bearer_token(&headers)?;
    let records = runtime
        .run(move || workflow.list(&token, query.limit, query.offset))
        .await?;
    Ok(Json(records))
}

async fn detail(
    State(CaseState { workflow, runtime }): State<CaseState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<CaseRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case_id(&id)?;
    let record = runtime.run(move || workflow.get(&token, id)).await?;
    Ok(Json(record))
}

async fn assign(
    State(CaseState { workflow, runtime }): State<CaseState>,
    headers: HeaderMap,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case_id(&id)?;
    let user_id = parse_user_id(&user_id)?;
    runtime
        .run(move || workflow.assign(&token, id, user_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(CaseState { workflow, runtime }): State<CaseState>,
    headers: HeaderMap,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_case_id(&id)?;
    let user_id = parse_user_id(&user_id)?;
    runtime
        .run(move || workflow.remove(&token, id, user_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn parse_case_id(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}

fn parse_user_id(value: &str) -> Result<UserId, ApiError> {
    Uuid::parse_str(value)
        .map(UserId::from_uuid)
        .map_err(|_| ApiError::invalid_user_id())
}
