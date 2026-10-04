use super::{query, request, response, HearingState};
use crate::{error::ApiError, request::bearer_token};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use domain::{
    cases::CaseId,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};
use serde_json::Value;

pub(super) async fn list(
    State(s): State<HearingState>,
    Path((case, resource)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<query::Page>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let resource = ResourceId::from_uuid(request::uuid(&resource)?);
    let query = input.map_err(|_| request::invalid())?.0.validate()?;
    let limit = query.limit();
    let after = query.after_id();
    let page = s
        .runtime
        .run(move || s.reads.list(&token, case, resource, query))
        .await?;
    Ok(Json(response::page(page, case, resource, limit, after)?))
}
pub(super) async fn detail(
    State(s): State<HearingState>,
    Path((case, resource, hearing)): Path<(String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    get(s, case, resource, hearing, None, headers, q).await
}
pub(super) async fn exact(
    State(s): State<HearingState>,
    Path((case, resource, hearing, revision)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    get(s, case, resource, hearing, Some(revision), headers, q).await
}
async fn get(
    s: HearingState,
    case: String,
    resource: String,
    hearing: String,
    revision: Option<String>,
    headers: HeaderMap,
    query: Option<String>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query::empty(query.as_deref())?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let resource = ResourceId::from_uuid(request::uuid(&resource)?);
    let id = ResourceHearingId::from_uuid(request::uuid(&hearing)?);
    let revision = revision
        .map(|v| ResourceHearingRevision::new(request::number(&v)?).map_err(|_| request::invalid()))
        .transpose()?;
    let row = s
        .runtime
        .run(move || s.reads.get(&token, case, resource, id, revision))
        .await?;
    Ok(Json(response::creation(row, case, resource, id, revision)?))
}
