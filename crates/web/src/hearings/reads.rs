use super::{parse_case, parse_id, parse_revision, projection, query, response, HearingState};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;

pub(super) async fn context(
    State(s): State<HearingState>,
    Path(case): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let row = s
        .runtime
        .run(move || s.workflow.context(&token, case))
        .await?;
    Ok(Json(projection::context(row, case)?))
}
pub(super) async fn list(
    State(s): State<HearingState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.list(&token, case, query))
        .await?;
    Ok(Json(response::page(row, case)?))
}
pub(super) async fn detail(
    State(s): State<HearingState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = parse_id(&id)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, id, None))
        .await?;
    Ok(Json(projection::detail(row, case, id, None)?))
}
pub(super) async fn exact(
    State(s): State<HearingState>,
    Path((case, id, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = parse_id(&id)?;
    let revision = parse_revision(&revision)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, id, Some(revision)))
        .await?;
    Ok(Json(projection::detail(row, case, id, Some(revision))?))
}
pub(super) async fn history(
    State(s): State<HearingState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::HistoryQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = parse_id(&id)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.history(&token, case, id, query))
        .await?;
    Ok(Json(response::history(row, case, id)?))
}
pub(super) async fn agenda(
    State(s): State<HearingState>,
    headers: HeaderMap,
    input: Result<Query<query::AgendaQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.agenda(&token, query))
        .await?;
    Ok(Json(response::agenda(row)?))
}
