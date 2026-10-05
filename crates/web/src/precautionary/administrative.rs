use super::{
    administrative_request as request, measure_request::invalid, primitives::*, projection,
    AdministrativeState, MAX_BODY,
};
use crate::{error::ApiError, procedural_facts::object::Object, request::bearer_token};
use application::measure_corrections::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::cases::CaseId;
use serde::de::DeserializeOwned;
use serde_json::Value;

async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    empty(body.uri().query())?;
    crate::request::json::read::<Object<T>>(
        body,
        MAX_BODY,
        "measure_administrative_body_too_large",
        "administrative measure JSON exceeds 128 KiB",
    )
    .await
    .map(|v| v.0)
}
fn bound(
    review: &MeasureAdministrativeReview,
    case: CaseId,
    command: &MeasureAdministrativeCommand,
) -> Result<(), ApiError> {
    if review.case_id != case || &review.command != command {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(super) async fn prepare(
    State(s): State<AdministrativeState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    body: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let input = json::<request::Command>(body).await?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let command = input.validate(case).map_err(|_| invalid())?;
                let review = s.workflow.prepare(&token, case, command.clone())?;
                bound(&review, case, &command)?;
                projection::admin_review(&review, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn submit(
    State(s): State<AdministrativeState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let input = json::<request::Submission>(body).await?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let (command, confirmation) = input.validate(case).map_err(|_| invalid())?;
                let operation = s
                    .workflow
                    .submit(&token, case, command.clone(), confirmation)?;
                let review = &operation.capture.review;
                bound(review, case, &command)?;
                if review.submission_digest != confirmation.submission_digest
                    || review.review_digest != confirmation.review_digest
                {
                    return Err(ApiError::internal());
                }
                projection::bound_admin(&operation, case, Some(command.operation_id))?;
                projection::admin_operation(&operation, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(value)))
}
