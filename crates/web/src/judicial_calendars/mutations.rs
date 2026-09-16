use super::{parse_id, query, request, response, CalendarState, MAX_BODY_BYTES};
use crate::{error::ApiError, request::bearer_token};
use application::judicial_calendars::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
pub(super) async fn prepare(
    State(s): State<CalendarState>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let command = json::<request::Command>(request).await?.validate()?;
    let expected = command.clone();
    let row = s
        .runtime
        .run(move || s.workflow.prepare(&token, command))
        .await?;
    Ok(Json(response::draft(row, &expected)?))
}
pub(super) async fn publish(
    State(s): State<CalendarState>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(s, None, JudicialCalendarAction::Publish, token, request).await
}
pub(super) async fn replace(
    State(s): State<CalendarState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(
        s,
        Some(parse_id(&id)?),
        JudicialCalendarAction::Replace,
        token,
        request,
    )
    .await
}
pub(super) async fn retire(
    State(s): State<CalendarState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(
        s,
        Some(parse_id(&id)?),
        JudicialCalendarAction::Retire,
        token,
        request,
    )
    .await
}
async fn submit(
    s: CalendarState,
    id: Option<JudicialCalendarId>,
    action: JudicialCalendarAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, digest) = json::<request::Submission>(request).await?.validate()?;
    if command.action() != action || id.is_some_and(|id| id != command.calendar_id) {
        return Err(ApiError::invalid_body(
            "judicial_calendar_command_mismatch",
            "route, calendar and command action must agree",
        ));
    }
    let id = command.calendar_id;
    let operation = command.operation_id;
    let revision = command.result_revision()?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, command, digest))
        .await?;
    if row.receipt.operation_id != operation
        || row.receipt.action != action
        || row.receipt.submission_digest != digest
    {
        return Err(ApiError::internal());
    }
    Ok((
        StatusCode::CREATED,
        Json(response::detail(row, id, Some(revision))?),
    ))
}
async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    query::require_empty(request.uri().query())?;
    crate::request::json::read::<super::object::Object<T>>(
        request,
        MAX_BODY_BYTES,
        "judicial_calendar_body_too_large",
        "judicial calendar JSON exceeds 1 MiB",
    )
    .await
    .map(|v| v.0)
}
