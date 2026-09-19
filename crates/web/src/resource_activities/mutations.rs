use super::{case, id, projection, query, request, resource, ResourceActivityState};
use crate::{error::ApiError, procedural_facts::object::Object, request::bearer_token};
use application::resource_activities::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::cases::CaseId;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) async fn prepare(
    State(s): State<ResourceActivityState>,
    Path((c, r)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&c)?;
    let resource = resource(&r)?;
    let command = json::<request::Command>(body)
        .await?
        .validate(case, resource)?;
    let expected = command.clone();
    let row = s
        .runtime
        .run(move || s.workflow.prepare(&token, case, resource, command))
        .await?;
    Ok(Json(projection::draft(row, case, resource, &expected)?))
}
pub(super) async fn link(
    State(s): State<ResourceActivityState>,
    Path((c, r)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(s, case(&c)?, resource(&r)?, None, token, body).await
}
pub(super) async fn unlink(
    State(s): State<ResourceActivityState>,
    Path((c, r, a)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(s, case(&c)?, resource(&r)?, Some(id(&a)?), token, body).await
}
async fn submit(
    s: ResourceActivityState,
    case: CaseId,
    resource: ResourceId,
    route_id: Option<ResourceActivityId>,
    token: String,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, digest) = json::<request::Submission>(body)
        .await?
        .validate(case, resource)?;
    let action = if route_id.is_some() {
        ResourceActivityAction::Unlink
    } else {
        ResourceActivityAction::Link
    };
    if command.action() != action || route_id.is_some_and(|id| id != command.association_id) {
        return Err(request::invalid(
            "route and command association or action differ",
        ));
    }
    let expected = command.clone();
    let id = command.association_id;
    let revision = command
        .result_revision()
        .map_err(|_| request::invalid("association revision exhausted"))?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, case, resource, command, digest))
        .await?;
    if row.receipt.submission_digest != digest
        || resource_activity_command_from_detail(&row).map_err(|_| ApiError::internal())?
            != expected
    {
        return Err(ApiError::internal());
    }
    Ok((
        StatusCode::CREATED,
        Json(projection::detail(row, case, resource, id, Some(revision))?),
    ))
}
async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    query::require_empty(body.uri().query())?;
    crate::request::json::read::<Object<T>>(
        body,
        16 * 1024,
        "resource_activity_body_too_large",
        "resource activity JSON exceeds 16 KiB",
    )
    .await
    .map(|v| v.0)
}
