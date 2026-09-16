use super::{projection, query, response, scope, FactState, RoutePath, Scope};
use crate::{error::ApiError, request::bearer_token};
use application::procedural_facts::FactFamily;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::Value;

pub(super) async fn list(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    input: Result<Query<query::PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let value = match s.family {
        FactFamily::Resolution => {
            let query = input.resolution()?;
            let row = s
                .runtime
                .run(move || s.workflow.list_resolutions(&token, scope.case, query))
                .await?;
            response::resolutions(row, scope.case)?
        }
        FactFamily::Notification => {
            let parent = scope.parent.ok_or_else(ApiError::internal)?;
            let query = input.notification()?;
            let row = s
                .runtime
                .run(move || {
                    s.workflow
                        .list_notifications(&token, scope.case, parent, query)
                })
                .await?;
            response::notifications(row, scope.case, parent)?
        }
    };
    Ok(Json(value))
}
pub(super) async fn detail(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let scope = Scope::parse(s.family, &path)?;
    let target = scope.target()?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, scope.case, target, None))
        .await?;
    Ok(Json(projection::detail(row, scope.case, target, None)?))
}
pub(super) async fn exact(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    RawQuery(parameters): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(parameters.as_deref())?;
    let scope = Scope::parse(s.family, &path)?;
    let target = scope.target()?;
    let revision = scope::revision(path.revision.as_deref().ok_or_else(ApiError::internal)?)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, scope.case, target, Some(revision)))
        .await?;
    Ok(Json(projection::detail(
        row,
        scope.case,
        target,
        Some(revision),
    )?))
}
pub(super) async fn history(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    input: Result<Query<query::HistoryQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    let target = scope.target()?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.history(&token, scope.case, target, query))
        .await?;
    Ok(Json(response::history(row, scope.case, target)?))
}
