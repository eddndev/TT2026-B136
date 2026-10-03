use application::{identity::password_reset::ResetCompletion, ApplicationError};
use axum::{
    extract::{Request, State as ExtractState},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use zeroize::Zeroize;

use super::{dto, error::ResetError, RequestAdmission, State};

#[derive(Serialize)]
pub(super) struct Accepted {
    status: &'static str,
}

pub(super) async fn request(
    ExtractState(state): ExtractState<State>,
    request: Request,
) -> Result<(StatusCode, Json<Accepted>), ResetError> {
    let input: dto::RequestInput = dto::read(request).await?;
    let components = state.components.ok_or(ResetError::Unavailable)?;
    match components.requests.try_submit(input.email) {
        RequestAdmission::Accepted | RequestAdmission::Busy => {
            Ok((StatusCode::ACCEPTED, Json(Accepted { status: "accepted" })))
        }
        RequestAdmission::Unavailable => Err(ResetError::Unavailable),
    }
}

pub(super) async fn complete(
    ExtractState(state): ExtractState<State>,
    request: Request,
) -> Result<StatusCode, ResetError> {
    let mut input: dto::CompletionInput = dto::read(request).await?;
    let components = state.components.ok_or(ResetError::Unavailable)?;
    let token = dto::token(&input.token)?;
    input.token.zeroize();
    // The service owns policy enforcement. Only its known invalid byte range
    // permits a public password-policy response; arbitrary port text never does.
    let invalid_password_length = !(12..=1024).contains(&input.new_password.len());
    let result = state
        .runtime
        .run(move || {
            // Preserve the application result instead of using the generic HTTP map.
            Ok(components
                .completion
                .complete(token.as_ref(), &input.new_password))
        })
        .await
        .map_err(|_| ResetError::Uncertain)?;
    match result {
        Ok(ResetCompletion::Changed) => Ok(StatusCode::NO_CONTENT),
        Ok(ResetCompletion::Rejected) => Err(ResetError::Rejected),
        Err(ApplicationError::InvalidInput(_)) if invalid_password_length => {
            Err(ResetError::Password)
        }
        Err(_) => Err(ResetError::Uncertain),
    }
}
