use super::{
    decision_request as request,
    decision_router::{DecisionState, MAX_DECISION_BODY},
    measure_request::invalid,
    primitives::*,
    projection,
};
use crate::{error::ApiError, procedural_facts::object::Object, request::bearer_token};
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
        MAX_DECISION_BODY,
        "measure_decision_body_too_large",
        "measure decision JSON exceeds 4 MiB",
    )
    .await
    .map(|v| v.0)
}

pub(super) async fn prepare(
    State(s): State<DecisionState>,
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
                if review.case_id() != case || review.command() != &command {
                    return Err(ApiError::internal());
                }
                projection::decision_review(&review, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}

pub(super) async fn submit(
    State(s): State<DecisionState>,
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
                let receipt = s
                    .workflow
                    .submit(&token, case, command.clone(), confirmation)?;
                if receipt.command() != &command
                    || receipt.submission_digest() != confirmation.submission_digest
                    || receipt.review_digest() != confirmation.review_digest
                {
                    return Err(ApiError::internal());
                }
                projection::bound_decision(
                    &receipt,
                    case,
                    Some(command.decision_id),
                    Some(command.operation_id),
                )?;
                projection::decision_operation(&receipt, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(value)))
}
