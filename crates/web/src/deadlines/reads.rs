use super::{query, response, DeadlineState, RoutePath};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;
pub(super) async fn list(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let c = path.case_id()?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let q = input.validate()?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let row = s.workflow.list(&token, c, q.clone())?;
                response::page(row, c, &q)
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn detail(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let c = path.case_id()?;
    let id = path.id()?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let row = s.workflow.current(&token, c, id)?;
                response::current(row, c, id)
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn exact(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let c = path.case_id()?;
    let id = path.id()?;
    let r = path.revision()?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let row = s.workflow.get(&token, c, id, Some(r))?;
                response::detail(row, c, id, Some(r))
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn history(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    input: Result<Query<query::HistoryQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let c = path.case_id()?;
    let id = path.id()?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let q = input.validate()?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let row = s.workflow.history(&token, c, id, q)?;
                response::history(row, c, id, q)
            })())
        })
        .await??;
    Ok(Json(value))
}
