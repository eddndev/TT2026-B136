use super::{object, query, request, response, scope, DeadlineState, RoutePath, MAX_BODY_BYTES};
use crate::{error::ApiError, request::bearer_token};
use application::deadlines::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
pub(super) async fn prepare(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let c = path.case_id()?;
    let input = json::<request::Command>(request).await?;
    // Corpus reproduction and projection run within the shared blocking-work budget.
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let command = input.validate()?;
                let expected = command.clone().into_parts().0;
                if !scope::accepts(c, &expected) {
                    return Err(scope::mismatch());
                }
                let row = s.workflow.prepare(&token, c, command.clone())?;
                response::draft(row, c, &command)
            })())
        })
        .await??;
    Ok(Json(result))
}
pub(super) async fn register(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(s, path, None, DeadlineAction::Register, token, request).await
}
pub(super) async fn correct(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = path.id()?;
    submit(s, path, Some(id), DeadlineAction::Correct, token, request).await
}
pub(super) async fn attention(
    State(s): State<DeadlineState>,
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
        DeadlineAction::SetAttention,
        token,
        request,
    )
    .await
}
pub(super) async fn retire(
    State(s): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let id = path.id()?;
    submit(s, path, Some(id), DeadlineAction::Retire, token, request).await
}
async fn submit(
    s: DeadlineState,
    path: RoutePath,
    id: Option<DeadlineId>,
    action: DeadlineAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let c = path.case_id()?;
    let input = json::<request::Submission>(request).await?;
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let (command, digest) = input.validate()?;
                let expected = command.clone().into_parts().0;
                if !scope::accepts(c, &expected)
                    || expected.action() != action
                    || id.is_some_and(|id| id != expected.deadline_id)
                {
                    return Err(scope::mismatch());
                }
                let revision = expected.result_revision()?;
                let row = s.workflow.submit(&token, c, command.clone(), digest)?;
                response::submitted(&row, &command, digest)?;
                response::detail(row, c, expected.deadline_id, Some(revision))
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
        "deadline_body_too_large",
        "deadline JSON exceeds 1 MiB",
    )
    .await
    .map(|v| v.0)
}
