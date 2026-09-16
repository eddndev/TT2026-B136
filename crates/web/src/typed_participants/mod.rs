//! HTTP representations and routes for reviewed case identities and roles.

mod body;
mod profile;
pub(crate) mod projection;
mod proposal;
mod response;
mod review;
mod subjects;
mod values;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::typed_participants::{ParticipantExpectation, TypedParticipantWorkflow};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use domain::cases::CaseId;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
struct ParticipantState {
    workflow: Arc<dyn TypedParticipantWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn TypedParticipantWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/cases/:case/participants/proposals/review",
            post(review_participant),
        )
        .route(
            "/api/v1/cases/:case/participants/proposals/prepare",
            post(prepare_participant),
        )
        .route(
            "/api/v1/cases/:case/participants/proposals/commit",
            post(commit_participant),
        )
        .route(
            "/api/v1/cases/:case/participants/:id/revisions/:revision/credential",
            get(credential),
        )
        .merge(subjects::router())
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(ParticipantState { workflow, runtime })
}
async fn review_participant(
    State(s): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let input = body::json::<proposal::ReviewRequest>(request).await?;
    let input = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.review_participant(&token, case, input))
        .await?;
    Ok(Json(response::review(case, row)))
}
async fn prepare_participant(
    State(s): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let input = body::json::<proposal::Preparation>(request).await?;
    let input = input.validate()?;
    let row = s
        .runtime
        .run(move || s.workflow.prepare_participant(&token, case, input))
        .await?;
    Ok(Json(response::draft(case, row)))
}
async fn commit_participant(
    State(s): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let input = body::json::<proposal::Submission>(request).await?;
    let input = input.validate()?;
    let status = if matches!(
        input.prepared.proposal.expected_participant(),
        ParticipantExpectation::Absent
    ) {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let row = s
        .runtime
        .run(move || s.workflow.submit_participant(&token, case, input))
        .await?;
    Ok((status, Json(projection::detail(row)?)))
}
async fn credential(
    State(s): State<ParticipantState>,
    Path((case, id, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let id = application::typed_participants::ParticipantId::from_uuid(parse_uuid(
        &id,
        "invalid_participant_id",
    )?);
    let revision =
        application::typed_participants::ParticipantRevision::new(parse_revision(&revision)?)
            .map_err(application::ApplicationError::from)?;
    let row = s
        .runtime
        .run(move || s.workflow.credential(&token, case, id, revision))
        .await?;
    Ok(Json(response::credential(row)?))
}
fn parse_case(value: &str) -> Result<CaseId, ApiError> {
    Uuid::parse_str(value)
        .map(CaseId::from_uuid)
        .map_err(|_| ApiError::invalid_case_id())
}
fn parse_uuid(value: &str, code: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| ApiError::invalid_body(code, "identifier must be a UUID"))
}
fn parse_revision(value: &str) -> Result<u32, ApiError> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::invalid_body(
            "invalid_revision",
            "revision must be a positive integer",
        ));
    }
    value.parse::<u32>().ok().filter(|v| *v > 0).ok_or_else(|| {
        ApiError::invalid_body("invalid_revision", "revision must be a positive integer")
    })
}

#[cfg(test)]
mod body_tests;
#[cfg(test)]
mod credential_tests;
#[cfg(test)]
mod proposal_tests;
#[cfg(test)]
mod route_support;
#[cfg(test)]
mod route_tests;
#[cfg(test)]
mod value_tests;
