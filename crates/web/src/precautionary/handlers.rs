use super::{primitives::*, projection, request, HearingState, MAX_BODY};
use crate::{error::ApiError, procedural_facts::object::Object, request::bearer_token};
use application::precautionary_hearings::*;
use axum::{
    extract::{Path, RawQuery, Request, State},
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
        "precautionary_hearing_body_too_large",
        "precautionary hearing JSON exceeds 128 KiB",
    )
    .await
    .map(|v| v.0)
}
pub(super) fn bound_review(
    review: &PrecautionaryHearingReview,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
) -> Result<(), ApiError> {
    if review.case_id != case
        || &review.command != command
        || review.result_revision
            != command
                .result_revision()
                .map_err(|_| ApiError::internal())?
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(super) async fn context(
    State(s): State<HearingState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let context = s.context.get(&token, case)?;
                if context.material().case_id != case {
                    return Err(ApiError::internal());
                }
                projection::context(&context, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn prepare(
    State(s): State<HearingState>,
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
                let command = input.validate(case)?;
                let review = s.workflow.prepare(&token, case, command.clone())?;
                bound_review(&review, case, &command)?;
                projection::review(&review, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn submit(
    State(s): State<HearingState>,
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
                let (command, confirmation) = input.validate(case)?;
                let operation = s
                    .workflow
                    .submit(&token, case, command.clone(), confirmation)?;
                let review = &operation.capture.review;
                bound_review(review, case, &command)?;
                if review.submission_digest != confirmation.submission_digest
                    || review.review_digest != confirmation.review_digest
                {
                    return Err(ApiError::internal());
                }
                super::reads::bound(
                    &operation,
                    case,
                    Some(command.hearing_id),
                    None,
                    Some(command.operation_id),
                )?;
                projection::operation(&operation, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(value)))
}
