//! Identity endpoints delegate authentication and authorization to use cases.

use super::AppState;
use crate::dto::{
    ChallengeCodeRequest, CreateUserRequest, CredentialsRequest, EnrollmentResponse,
    LoginChallengeResponse, PrincipalResponse, SessionResponse, SessionStatusResponse,
};
use crate::{error::ApiError, request::bearer_token};
use application::ApplicationError;
use axum::{
    body::Bytes,
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use domain::identity::Role;
use std::str::FromStr;

pub(super) async fn bootstrap_owner(
    State(state): State<AppState>,
    Json(request): Json<CredentialsRequest>,
) -> Result<(StatusCode, Json<EnrollmentResponse>), ApiError> {
    let identity = state.identity.clone();
    let result = state
        .runtime
        .run(move || identity.bootstrap_owner(&request.email, &request.password))
        .await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

pub(super) async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<EnrollmentResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let role = Role::from_str(&request.role)
        .map_err(|_| ApplicationError::InvalidInput("invalid role".to_string()))?;
    let identity = state.identity.clone();
    let result = state
        .runtime
        .run(move || identity.create_user(&token, &request.email, &request.password, role))
        .await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

pub(super) async fn start_login(
    State(state): State<AppState>,
    Json(request): Json<CredentialsRequest>,
) -> Result<Json<LoginChallengeResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || identity.start_login(&request.email, &request.password))
            .await?
            .into(),
    ))
}

pub(super) async fn complete_totp(
    State(state): State<AppState>,
    axum::Extension(access): axum::Extension<crate::access_log::AccessAttempt>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || {
                let attempt =
                    identity.complete_totp_observed(&request.challenge_token, &request.code);
                access.record(&attempt);
                attempt.result
            })
            .await?
            .into(),
    ))
}

pub(super) async fn complete_recovery(
    State(state): State<AppState>,
    axum::Extension(access): axum::Extension<crate::access_log::AccessAttempt>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || {
                let attempt =
                    identity.complete_recovery_observed(&request.challenge_token, &request.code);
                access.record(&attempt);
                attempt.result
            })
            .await?
            .into(),
    ))
}

pub(super) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    let identity = state.identity.clone();
    state.runtime.run(move || identity.logout(&token)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PrincipalResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || identity.authenticate(&token))
            .await?
            .into(),
    ))
}

pub(super) async fn session_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Result<Json<SessionStatusResponse>, ApiError> {
    let token = session_request_token(&headers, query.as_deref(), &body)?;
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || identity.session_status(&token))
            .await?
            .into(),
    ))
}

pub(super) async fn record_activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Result<Json<SessionStatusResponse>, ApiError> {
    let token = session_request_token(&headers, query.as_deref(), &body)?;
    let identity = state.identity.clone();
    Ok(Json(
        state
            .runtime
            .run(move || identity.record_activity(&token))
            .await?
            .into(),
    ))
}

fn session_request_token(
    headers: &HeaderMap,
    query: Option<&str>,
    body: &[u8],
) -> Result<String, ApiError> {
    let token = bearer_token(headers)?;
    if query.is_some() || !body.is_empty() {
        return Err(ApiError::invalid_body(
            "invalid_session_request",
            "session requests require an empty body and no query string",
        ));
    }
    Ok(token)
}
