use super::{draft, request, response, HearingState, MAX_BODY};
use crate::{error::ApiError, procedural_facts::object::Object, request::bearer_token};
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::{cases::CaseId, procedural_resources::ResourceId};
use serde::de::DeserializeOwned;
use serde_json::Value;

async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    if body.uri().query().is_some_and(|q| !q.is_empty()) {
        return Err(request::invalid());
    }
    crate::request::json::read::<Object<T>>(
        body,
        MAX_BODY,
        "resource_hearing_body_too_large",
        "resource hearing JSON exceeds 64 KiB",
    )
    .await
    .map(|v| v.0)
}
pub(super) async fn prepare(
    State(s): State<HearingState>,
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
                let value = s
                    .workflow
                    .prepare(&token, case, resource, command.clone())?;
                draft::project(value, case, resource, &command)
            })())
        })
        .await??;
    Ok(Json(result))
}
pub(super) async fn submit(
    State(s): State<HearingState>,
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
