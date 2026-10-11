use axum::{
    extract::{Request, State},
    Json,
};
use serde_json::{json, Value};

use super::{error::InvitationError, input, output, InvitationState};

pub(super) async fn prepare(
    State(state): State<InvitationState>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let input = input::read::<input::Recipient>(request).await?.material()?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.prepare(input.token.as_ref(), input.operation, &input.password)))
        .await
        .map_err(|_| InvitationError::Uncertain)?
        .map_err(InvitationError::recipient)?;
    Ok(Json(output::enrollment(value)?))
}

pub(super) async fn finish(
    State(state): State<InvitationState>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let (input, code) = input::read::<input::Finish>(request).await?.material()?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || {
            Ok(workflow.finish(
                input.token.as_ref(),
                input.operation,
                &input.password,
                &code,
            ))
        })
        .await
        .map_err(|_| InvitationError::Uncertain)?
        .map_err(InvitationError::recipient)?;
    Ok(Json(output::receipt(value)?))
}

pub(super) async fn receipt(
    State(state): State<InvitationState>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let input = input::read::<input::Recipient>(request).await?.material()?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.receipt(input.token.as_ref(), input.operation, &input.password)))
        .await
        .map_err(|_| InvitationError::Uncertain)?
        .map_err(InvitationError::recipient)?;
    Ok(Json(
        json!({"receipt":value.map(output::receipt).transpose()?}),
    ))
}
