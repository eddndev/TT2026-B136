use super::{parse_id, parse_revision, query, response, CalendarState};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;
pub(super) async fn list(
    State(s): State<CalendarState>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let q = input.validate()?;
    let row = s.runtime.run(move || s.workflow.list(&token, q)).await?;
    Ok(Json(response::page(row)))
}
pub(super) async fn detail(
    State(s): State<CalendarState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let id = parse_id(&id)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, id, None))
        .await?;
    Ok(Json(response::detail(row, id, None)?))
}
pub(super) async fn exact(
    State(s): State<CalendarState>,
    Path((id, revision)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let id = parse_id(&id)?;
    let revision = parse_revision(&revision)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, id, Some(revision)))
        .await?;
    Ok(Json(response::detail(row, id, Some(revision))?))
}
pub(super) async fn history(
    State(s): State<CalendarState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Query<query::HistoryQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_id(&id)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let q = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.history(&token, id, q))
        .await?;
    Ok(Json(response::history(row, id)?))
}
pub(super) async fn days(
    State(s): State<CalendarState>,
    Path((id, revision)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::DaysQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let id = parse_id(&id)?;
    let revision = parse_revision(&revision)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let q = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.days(&token, id, revision, q))
        .await?;
    Ok(Json(response::days(row, id, revision, q)?))
}
