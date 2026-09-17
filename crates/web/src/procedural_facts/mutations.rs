use super::{projection, query, request, response, FactState, RoutePath, Scope, BODY_LIMIT};
use crate::{error::ApiError, request::bearer_token};
use application::procedural_facts::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) async fn prepare(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    let command = json::<request::Command>(request).await?.validate()?;
    if !scope.accepts(&command) {
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
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    submit(s, scope, FactAction::Record, token, request).await
}
pub(super) async fn correct(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    scope.target()?;
    submit(s, scope, FactAction::Correct, token, request).await
}
pub(super) async fn withdraw(
    State(s): State<FactState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let scope = Scope::parse(s.family, &path)?;
    scope.target()?;
    submit(s, scope, FactAction::Withdraw, token, request).await
}
async fn submit(
    s: FactState,
    scope: Scope,
    action: FactAction,
    token: String,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, digest) = json::<request::Submission>(request).await?.validate()?;
    if !scope.accepts(&command) || command.action() != action {
        return Err(mismatch());
    }
    let expected = command.clone();
    let revision = command
        .result_revision()
        .map_err(application::ApplicationError::from)?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, scope.case, command, digest))
        .await?;
    let metadata = row.snapshot.metadata();
    if metadata.receipt.operation_id != expected.operation_id()
        || metadata.receipt.action != action
        || metadata.receipt.submission_digest != digest
        || metadata.receipt.expected_revision != expected.expected_revision()
        || metadata.reason.as_ref() != expected.reason()
        || !submitted_values_match(&expected, &row.snapshot)
    {
        return Err(ApiError::internal());
    }
    Ok((
        StatusCode::CREATED,
        Json(projection::detail(
            row,
            scope.case,
            expected.target(),
            Some(revision),
        )?),
    ))
}
fn submitted_values_match(
    command: &ProceduralFactCommand,
    snapshot: &ProceduralFactSnapshot,
) -> bool {
    match (command, snapshot) {
        (ProceduralFactCommand::Resolution(c), ProceduralFactSnapshot::Resolution(s)) => {
            c.change().values().is_none_or(|v| v == &s.values)
        }
        (ProceduralFactCommand::Notification(c), ProceduralFactSnapshot::Notification(s)) => {
            c.change().values().is_none_or(|v| v == &s.values)
        }
        _ => false,
    }
}
fn mismatch() -> ApiError {
    ApiError::invalid_body(
        "procedural_fact_command_mismatch",
        "route, family, identity, parent and action must agree",
    )
}
async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    query::require_empty(request.uri().query())?;
    crate::request::json::read::<T>(
        request,
        BODY_LIMIT,
        "procedural_fact_body_too_large",
        "procedural fact JSON exceeds 512 KiB",
    )
    .await
}
