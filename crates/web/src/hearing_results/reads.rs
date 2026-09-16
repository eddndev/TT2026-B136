use super::{parse_id, parse_revision, projection, query, response, ResultState, Scope};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;
pub(super) async fn list(
    State(s): State<ResultState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.list(&token, scope.case, scope.hearing, query))
        .await?;
    Ok(Json(response::page(row, scope.case, scope.hearing)?))
}
pub(super) async fn detail(
    State(s): State<ResultState>,
    Path((case, hearing, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let scope = Scope::parse(&case, &hearing)?;
    let id = parse_id(&id)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, scope.case, scope.hearing, id, None))
        .await?;
    Ok(Json(projection::detail(
        row,
        scope.case,
        scope.hearing,
        id,
        None,
    )?))
}
pub(super) async fn exact(
    State(s): State<ResultState>,
    Path((case, hearing, id, revision)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let scope = Scope::parse(&case, &hearing)?;
    let id = parse_id(&id)?;
    let revision = parse_revision(&revision)?;
    let row = s
        .runtime
        .run(move || {
            s.workflow
                .get(&token, scope.case, scope.hearing, id, Some(revision))
        })
        .await?;
    Ok(Json(projection::detail(
        row,
        scope.case,
        scope.hearing,
        id,
        Some(revision),
    )?))
}
pub(super) async fn history(
    State(s): State<ResultState>,
    Path((case, hearing, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::HistoryQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    let id = parse_id(&id)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || {
            s.workflow
                .history(&token, scope.case, scope.hearing, id, query)
        })
        .await?;
    Ok(Json(response::history(row, scope.case, scope.hearing, id)?))
}
