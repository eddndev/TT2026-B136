//! Review and recover a declared result with its configured deadline consequence.
mod request;
mod response;

use crate::{
    error::ApiError, procedural_facts::object::Object, request::bearer_token, runtime::HttpRuntime,
};
use application::hearing_derived_deadlines::HearingDerivedDeadlineWorkflow;
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use domain::{cases::CaseId, hearings::HearingId};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

const MAX_BODY: usize = 1024 * 1024;

#[derive(Clone)]
struct DerivedState {
    workflow: Arc<dyn HearingDerivedDeadlineWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(
    workflow: Arc<dyn HearingDerivedDeadlineWorkflow>,
    runtime: HttpRuntime,
) -> Router {
    let base = "/api/v1/cases/:case/hearings/:hearing/results/derived-deadline";
    Router::new()
        .route(&format!("{base}/prepare"), post(prepare))
        .route(&format!("{base}/submit"), post(submit))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(DerivedState { workflow, runtime })
}

async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    if body.uri().query().is_some() {
        return Err(request::invalid());
    }
    crate::request::json::read::<Object<T>>(
        body,
        MAX_BODY,
        "hearing_derived_deadline_body_too_large",
        "hearing derived deadline JSON exceeds 1 MiB",
    )
    .await
    .map(|value| value.0)
}

async fn prepare(
    State(s): State<DerivedState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let hearing = HearingId::from_uuid(request::uuid(&hearing)?);
    let input = json::<request::Command>(body).await?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let command = input.validate(case, hearing)?;
                let review = s.workflow.prepare(&token, case, command.clone())?;
                response::review(review, case, &command)
            })())
        })
        .await??;
    Ok(Json(value))
}

async fn submit(
    State(s): State<DerivedState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let hearing = HearingId::from_uuid(request::uuid(&hearing)?);
    let input = json::<request::Submission>(body).await?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let (command, digest) = input.validate(case, hearing)?;
                let record = s.workflow.submit(&token, case, command.clone(), digest)?;
                response::record(record, case, &command, Some(digest))
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(value)))
}
