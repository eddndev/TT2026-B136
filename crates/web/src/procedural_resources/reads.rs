use super::{case, id, projection, query, revision, ResourceState};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
pub(super) async fn list(
    State(s): State<ResourceState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    input: Result<Query<query::Page>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&path)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = s
        .runtime
        .run(move || s.workflow.list(&token, case, query))
        .await?;
    if page.resources.len() > query.limit() as usize
        || page.has_more != page.next_after_id.is_some()
        || page.has_more && page.resources.last().map(|r| r.id) != page.next_after_id
        || page.resources.iter().any(|r| {
            query
                .after_id()
                .is_some_and(|id| r.id.as_uuid() <= id.as_uuid())
                || query.kind().is_some_and(|k| r.values.kind() != k)
                || query.status().is_some_and(|status| r.status != status)
        })
        || page
            .resources
            .windows(2)
            .any(|r| r[0].id.as_uuid() >= r[1].id.as_uuid())
    {
        return Err(ApiError::internal());
    }
    let rows = page
        .resources
        .into_iter()
        .map(|row| {
            let id = row.id;
            projection::detail(row, case, id, None)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(
        json!({"resources":rows,"has_more":page.has_more,"next_after_id":page.next_after_id.map(|r|r.to_string())}),
    ))
}
pub(super) async fn detail(
    State(s): State<ResourceState>,
    Path((c, r)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(q.as_deref())?;
    let case = case(&c)?;
    let id = id(&r)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, id, None))
        .await?;
    Ok(Json(projection::detail(row, case, id, None)?))
}
pub(super) async fn exact(
    State(s): State<ResourceState>,
    Path((c, r, v)): Path<(String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(q.as_deref())?;
    let case = case(&c)?;
    let id = id(&r)?;
    let revision = revision(&v)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, id, Some(revision)))
        .await?;
    Ok(Json(projection::detail(row, case, id, Some(revision))?))
}
pub(super) async fn history(
    State(s): State<ResourceState>,
    Path((c, r)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::History>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&c)?;
    let id = id(&r)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = s
        .runtime
        .run(move || s.workflow.history(&token, case, id, query))
        .await?;
    if page.revisions.len() > query.limit() as usize
        || page.has_more != page.next_before_revision.is_some()
        || page.has_more && page.revisions.last().map(|r| r.revision) != page.next_before_revision
        || page
            .revisions
            .iter()
            .any(|r| query.before_revision().is_some_and(|v| r.revision >= v))
        || page
            .revisions
            .windows(2)
            .any(|r| r[0].revision <= r[1].revision)
    {
        return Err(ApiError::internal());
    }
    let rows = page
        .revisions
        .into_iter()
        .map(|row| projection::detail(row, case, id, None))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(
        json!({"revisions":rows,"has_more":page.has_more,"next_before_revision":page.next_before_revision.map(|r|r.get())}),
    ))
}
