use super::{case, current, id, projection, query, request, resource, ResourceActivityState};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

pub(super) async fn list(
    State(s): State<ResourceActivityState>,
    Path((c, r)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::Page>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&c)?;
    let resource = resource(&r)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = s
        .runtime
        .run(move || s.workflow.list(&token, case, resource, query))
        .await?;
    if page.associations.len() > query.limit() as usize
        || page.has_more != page.next_after_id.is_some()
        || page.has_more
            && (page.associations.len() != query.limit() as usize
                || page.associations.last().map(|v| v.association.id) != page.next_after_id)
        || page.associations.iter().any(|v| {
            query
                .after_id()
                .is_some_and(|id| v.association.id.as_uuid() <= id.as_uuid())
                || query
                    .kind()
                    .is_some_and(|k| v.association.selection.target.kind() != k)
                || query
                    .status()
                    .is_some_and(|status| v.association.status != status)
        })
        || page.associations.windows(2).any(|v| {
            v[0].association.id.as_uuid() >= v[1].association.id.as_uuid()
                || v[0].checked_at != v[1].checked_at
        })
    {
        return Err(ApiError::internal());
    }
    let rows = page
        .associations
        .into_iter()
        .map(|row| {
            let id = row.association.id;
            current::view(row, case, resource, id, None)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(
        json!({"case_id":case,"resource_id":resource.to_string(),"associations":rows,"has_more":page.has_more,"next_after_id":page.next_after_id.map(|v|v.to_string())}),
    ))
}
pub(super) async fn detail(
    State(s): State<ResourceActivityState>,
    Path((c, r, a)): Path<(String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(q.as_deref())?;
    let case = case(&c)?;
    let resource = resource(&r)?;
    let id = id(&a)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, resource, id, None))
        .await?;
    Ok(Json(current::view(row, case, resource, id, None)?))
}
pub(super) async fn exact(
    State(s): State<ResourceActivityState>,
    Path((c, r, a, v)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::require_empty(q.as_deref())?;
    let case = case(&c)?;
    let resource = resource(&r)?;
    let id = id(&a)?;
    let revision = request::revision(request::number(&v)?)?;
    let row = s
        .runtime
        .run(move || s.workflow.get(&token, case, resource, id, Some(revision)))
        .await?;
    Ok(Json(current::view(
        row,
        case,
        resource,
        id,
        Some(revision),
    )?))
}
pub(super) async fn history(
    State(s): State<ResourceActivityState>,
    Path((c, r, a)): Path<(String, String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::History>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&c)?;
    let resource = resource(&r)?;
    let id = id(&a)?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = s
        .runtime
        .run(move || s.workflow.history(&token, case, resource, id, query))
        .await?;
    if page.revisions.len() > query.limit() as usize
        || page.has_more != page.next_before_revision.is_some()
        || page.has_more
            && (page.revisions.len() != query.limit() as usize
                || page.revisions.last().map(|r| r.revision) != page.next_before_revision)
        || page
            .revisions
            .iter()
            .any(|r| query.before_revision().is_some_and(|v| r.revision >= v))
        || page.revisions.windows(2).any(|r| {
            r[0].revision.get().checked_sub(1) != Some(r[1].revision.get())
                || r[0].receipt.previous.is_none_or(|p| {
                    p.revision != r[1].revision || p.capture_digest != r[1].receipt.capture_digest
                })
                || r[0].recorded_at < r[1].recorded_at
                || r[0].selection != r[1].selection
                || r[0].sources != r[1].sources
        })
    {
        return Err(ApiError::internal());
    }
    let rows = page
        .revisions
        .into_iter()
        .map(|row| projection::detail(row, case, resource, id, None))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(
        json!({"case_id":case,"resource_id":resource.to_string(),"association_id":id.to_string(),"revisions":rows,"has_more":page.has_more,"next_before_revision":page.next_before_revision.map(|r|r.get())}),
    ))
}
