use super::{case, id, projection, query, request, ResourceState};
use crate::{error::ApiError, request::bearer_token};
use application::procedural_resources::*;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::cases::CaseId;
use serde::de::DeserializeOwned;
use serde_json::Value;
pub(super) async fn prepare(
    State(s): State<ResourceState>,
    Path(c): Path<String>,
    headers: HeaderMap,
    body: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&c)?;
    let command = json::<request::Command>(body).await?.validate()?;
    let expected = command.clone();
    let row = s
        .runtime
        .run(move || s.workflow.prepare(&token, case, command))
        .await?;
    Ok(Json(projection::draft(row, case, &expected)?))
}
pub(super) async fn create(
    State(s): State<ResourceState>,
    Path(c): Path<String>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(
        s,
        case(&c)?,
        None,
        None,
        ResourceAction::Register,
        token,
        body,
    )
    .await
}
macro_rules! handler {
    ($name:ident,$action:ident) => {
        pub(super) async fn $name(
            State(s): State<ResourceState>,
            Path((c, r)): Path<(String, String)>,
            headers: HeaderMap,
            body: Request,
        ) -> Result<(StatusCode, Json<Value>), ApiError> {
            let token = bearer_token(&headers)?;
            submit(
                s,
                case(&c)?,
                Some(id(&r)?),
                None,
                ResourceAction::$action,
                token,
                body,
            )
            .await
        }
    };
}
handler!(correct, Correct);
handler!(record_act, RecordAct);
handler!(archive, Archive);
handler!(reactivate, Reactivate);
pub(super) async fn correct_act(
    State(s): State<ResourceState>,
    Path((c, r, a)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(&headers)?;
    submit(
        s,
        case(&c)?,
        Some(id(&r)?),
        Some(ResourceActId::from_uuid(request::uuid(&a)?)),
        ResourceAction::CorrectAct,
        token,
        body,
    )
    .await
}
async fn submit(
    s: ResourceState,
    case: CaseId,
    id: Option<ResourceId>,
    act: Option<ResourceActId>,
    action: ResourceAction,
    token: String,
    body: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (command, digest) = json::<request::Submission>(body).await?.validate()?;
    let selected_act = match &command.change {
        ResourceChange::CorrectAct { act_id, .. } => Some(*act_id),
        _ => None,
    };
    if command.action() != action
        || id.is_some_and(|id| id != command.resource_id)
        || act != selected_act
    {
        return Err(ApiError::invalid_body(
            "procedural_resource_command_mismatch",
            "route and command identity or action differ",
        ));
    }
    let expected = command.clone();
    let id = command.resource_id;
    let revision = command.result_revision()?;
    let row = s
        .runtime
        .run(move || s.workflow.submit(&token, case, command, digest))
        .await?;
    if row.receipt.operation_id != expected.operation_id
        || row.receipt.action != action
        || row.receipt.expected_revision != expected.expected_revision()
        || row.receipt.submission_digest != digest
        || row.reason.as_ref() != expected.reason()
    {
        return Err(ApiError::internal());
    }
    match (&expected.change, row.act.as_ref()) {
        (ResourceChange::Register { values } | ResourceChange::Correct { values, .. }, _)
            if *values != row.values =>
        {
            return Err(ApiError::internal())
        }
        (
            ResourceChange::RecordAct { act_id, values, .. }
            | ResourceChange::CorrectAct { act_id, values, .. },
            Some(capture),
        ) if *act_id != capture.id || *values != capture.values => return Err(ApiError::internal()),
        _ => {}
    }
    if let ResourceChange::CorrectAct {
        expected_act_revision,
        ..
    } = &expected.change
    {
        if row.act.as_ref().map(|a| a.revision.get()) != expected_act_revision.get().checked_add(1)
        {
            return Err(ApiError::internal());
        }
    }
    Ok((
        StatusCode::CREATED,
        Json(projection::detail(row, case, id, Some(revision))?),
    ))
}
async fn json<T: DeserializeOwned>(body: Request) -> Result<T, ApiError> {
    query::require_empty(body.uri().query())?;
    crate::request::json::read::<super::object::Object<T>>(
        body,
        512 * 1024,
        "procedural_resource_body_too_large",
        "procedural resource JSON exceeds 512 KiB",
    )
    .await
    .map(|v| v.0)
}
