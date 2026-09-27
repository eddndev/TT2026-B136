//! Atomic creation of a calculated deadline and its exact resource association.
mod request;
mod response;
use crate::{
    error::ApiError, procedural_facts::object::Object, request::bearer_token, runtime::HttpRuntime,
};
use application::resource_deadlines::ResourceDeadlineWorkflow;
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use domain::{cases::CaseId, procedural_resources::ResourceId};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
const MAX_BODY: usize = 1024 * 1024;
#[derive(Clone)]
struct ContextState {
    workflow: Arc<dyn ResourceDeadlineWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn ResourceDeadlineWorkflow>, runtime: HttpRuntime) -> Router {
    let base = "/api/v1/cases/:case/procedural-resources/:id/activities/deadlines";
    Router::new()
        .route(&format!("{base}/prepare"), post(prepare))
        .route(&format!("{base}/submit"), post(submit))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(ContextState { workflow, runtime })
}
async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    if body.uri().query().is_some_and(|q| !q.is_empty()) {
        return Err(request::invalid());
    }
    crate::request::json::read::<Object<T>>(
        body,
        MAX_BODY,
        "resource_deadline_body_too_large",
        "contextual deadline JSON exceeds 1 MiB",
    )
    .await
    .map(|v| v.0)
}
async fn prepare(
    State(s): State<ContextState>,
    Path((case, resource)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let resource = ResourceId::from_uuid(request::uuid(&resource)?);
    let input = json::<request::Command>(body).await?;
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let command = input.validate(case, resource)?;
                let draft = s
                    .workflow
                    .prepare(&token, case, resource, command.clone())?;
                response::draft(draft, case, resource, &command)
            })())
        })
        .await??;
    Ok(Json(result))
}
async fn submit(
    State(s): State<ContextState>,
    Path((case, resource)): Path<(String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(request::uuid(&case)?);
    let resource = ResourceId::from_uuid(request::uuid(&resource)?);
    let input = json::<request::Submission>(body).await?;
    let result = s
        .runtime
        .run(move || {
            Ok((|| {
                let (command, digest) = input.validate(case, resource)?;
                let value = s
                    .workflow
                    .submit(&token, case, resource, command.clone(), digest)?;
                response::submitted(value, case, resource, &command, digest)
            })())
        })
        .await??;
    Ok((StatusCode::CREATED, Json(result)))
}
