use super::{parse_id, projection, query, request, response, ResultState, Scope};
use crate::{error::ApiError, request::bearer_token};
use application::hearing_results::{HearingResultAction, HearingResultId};
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
pub(super) async fn prepare(
    State(s): State<ResultState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    let command = json::<request::Command>(request).await?.validate()?;
    if command.hearing_id != scope.hearing {
        return Err(mismatch());
    }
    let expected = command.clone();
    let row = s
        .runtime
        .run(move || s.workflow.prepare(&token, scope.case, command))
        .await?;
    Ok(Json(response::draft(row, scope.case, &expected)?))
}
pub(super) async fn create(
    State(s): State<ResultState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    submit(s, scope, None, HearingResultAction::Record, token, request).await
}
pub(super) async fn correct(
    State(s): State<ResultState>,
    Path((case, hearing, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    let id = parse_id(&id)?;
    submit(
        s,
        scope,
        Some(id),
        HearingResultAction::Correct,
        token,
        request,
    )
    .await
}
pub(super) async fn withdraw(
    State(s): State<ResultState>,
    Path((case, hearing, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(&case, &hearing)?;
    let id = parse_id(&id)?;
    submit(
        s,
        scope,
        Some(id),
        HearingResultAction::Withdraw,
        token,
        request,
    )
    .await
}
async fn submit(
    s: ResultState,
    scope: Scope,
    id: Option<HearingResultId>,
    action: HearingResultAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, digest) = json::<request::Submission>(request).await?.validate()?;
    if command.hearing_id != scope.hearing
        || command.action() != action
        || id.is_some_and(|id| id != command.result_id)
    {
        return Err(mismatch());
    }
    let id = command.result_id;
    let operation = command.operation_id;
    let revision = command.result_revision()?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, scope.case, command, digest))
        .await?;
    if row.snapshot.receipt.operation_id != operation
        || row.snapshot.receipt.action != action
        || row.snapshot.receipt.submission_digest != digest
    {
        return Err(ApiError::internal());
    }
    Ok((
        StatusCode::CREATED,
        Json(projection::detail(
            row,
            scope.case,
            scope.hearing,
            id,
            Some(revision),
        )?),
    ))
}
fn mismatch() -> ApiError {
    ApiError::invalid_body(
        "hearing_result_command_mismatch",
        "route, hearing, result and command action must agree",
    )
}
async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    query::require_empty(request.uri().query())?;
    crate::request::json::read::<super::object::Object<T>>(
        request,
        512 * 1024,
        "hearing_result_body_too_large",
        "hearing result JSON exceeds 512 KiB",
    )
    .await
    .map(|value| value.0)
}
