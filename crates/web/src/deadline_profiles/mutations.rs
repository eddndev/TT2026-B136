use super::{object, query, request, response, scope, ProfileState, RoutePath, MAX_BODY_BYTES};
use crate::{error::ApiError, request::bearer_token};
use application::deadline_profiles::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
pub(super) async fn prepare(
    State(s): State<ProfileState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let c = path.collection()?;
    let input = json::<request::Command>(request).await?;
    // Corpus reproduction and projection run within the shared blocking-work budget.
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let command = input.validate()?;
                if !scope::accepts(c, &command) {
                    return Err(scope::mismatch());
                }
                let expected = command.clone();
                let row = s.workflow.prepare(&token, c, command)?;
                response::draft(row, c, &expected)
            })())
        })
        .await??;
    Ok(Json(result))
}
pub(super) async fn publish(
    State(s): State<ProfileState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(
        s,
        path,
        None,
        DeadlineProfileAction::Publish,
        token,
        request,
    )
    .await
}
pub(super) async fn replace(
    State(s): State<ProfileState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = path.id()?;
    submit(
        s,
        path,
        Some(id),
        DeadlineProfileAction::Replace,
        token,
        request,
    )
    .await
}
pub(super) async fn retire(
    State(s): State<ProfileState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = path.id()?;
    submit(
        s,
        path,
        Some(id),
        DeadlineProfileAction::Retire,
        token,
        request,
    )
    .await
}
async fn submit(
    s: ProfileState,
    path: RoutePath,
    id: Option<DeadlineProfileId>,
    action: DeadlineProfileAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let c = path.collection()?;
    let input = json::<request::Submission>(request).await?;
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let (command, digest) = input.validate()?;
                if !scope::accepts(c, &command)
                    || command.action() != action
                    || id.is_some_and(|id| id != command.profile_id)
                {
                    return Err(scope::mismatch());
                }
                let expected = command.clone();
                let revision = command.result_revision()?;
                let row = s.workflow.submit(&token, c, command, digest)?;
                response::submitted(&row, &expected, digest)?;
                response::detail(row, c, expected.profile_id, Some(revision))
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(result)))
}
async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    query::require_empty(request.uri().query())?;
    crate::request::json::read::<object::Object<T>>(
        request,
        MAX_BODY_BYTES,
        "deadline_profile_body_too_large",
        "deadline profile JSON exceeds 16 MiB",
    )
    .await
    .map(|v| v.0)
}
