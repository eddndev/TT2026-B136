use super::{parse_case, parse_id, projection, request, response, HearingState};
use crate::{error::ApiError, request::bearer_token};
use application::hearings::{HearingAction, HearingId};
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::cases::CaseId;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) async fn prepare(
    State(s): State<HearingState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let command = json::<request::Command>(request).await?.validate()?;
    let expected = command.clone();
    let row = s
        .runtime
        .run(move || s.workflow.prepare(&token, case, command))
        .await?;
    Ok(Json(response::draft(row, case, &expected)?))
}
pub(super) async fn create(
    State(s): State<HearingState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    submit(s, case, None, HearingAction::Schedule, token, request).await
}
pub(super) async fn replace(
    State(s): State<HearingState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = parse_id(&id)?;
    submit(s, case, Some(id), HearingAction::Replace, token, request).await
}
pub(super) async fn cancel(
    State(s): State<HearingState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = parse_id(&id)?;
    submit(s, case, Some(id), HearingAction::Cancel, token, request).await
}
async fn submit(
    s: HearingState,
    case: CaseId,
    id: Option<HearingId>,
    action: HearingAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, expected_digest) = json::<request::Submission>(request).await?.validate()?;
    if command.action() != action || id.is_some_and(|id| id != command.hearing_id) {
        return Err(ApiError::invalid_body(
            "hearing_command_mismatch",
            "route, hearing identifier and command action must agree",
        ));
    }
    let id = command.hearing_id;
    let operation = command.operation_id;
    let revision = command.result_revision()?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, case, command, expected_digest))
        .await?;
    if row.snapshot.receipt.operation_id != operation
        || row.snapshot.receipt.action != action
        || row.snapshot.receipt.submission_digest != expected_digest
    {
        return Err(ApiError::internal());
    }
    Ok((
        StatusCode::CREATED,
        Json(projection::detail(row, case, id, Some(revision))?),
    ))
}
async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read::<super::object::Object<T>>(
        request,
        64 * 1024,
        "hearing_body_too_large",
        "hearing JSON exceeds 64 KiB",
    )
    .await
    .map(|value| value.0)
}
