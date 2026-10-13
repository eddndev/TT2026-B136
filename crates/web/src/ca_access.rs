//! HTTP access for the separately configured CA operator.

use std::sync::Arc;

use application::{
    ca_access::{CaOperatorAccess, OperatorIdentity, OperatorSession},
    identity::SessionStatus,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, RawQuery, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Serialize;

use crate::{
    access_log::AccessAttempt,
    dto::{ChallengeCodeRequest, CredentialsRequest, LoginChallengeResponse},
    error::ApiError,
    request::bearer_token,
    runtime::{protect, HttpLimits, HttpRuntime},
};

#[derive(Clone)]
struct CaState {
    access: Arc<CaOperatorAccess>,
    runtime: HttpRuntime,
}

/// Builds bounded identity routes for the independently enrolled CA operator.
pub fn ca_access_router(access: Arc<CaOperatorAccess>, limits: HttpLimits) -> Router {
    let runtime = HttpRuntime::new(limits);
    protect(router(access, runtime.clone()), runtime)
}

fn router(access: Arc<CaOperatorAccess>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(start_login))
        .route("/api/v1/auth/mfa/totp", post(complete_totp))
        .route("/api/v1/auth/mfa/recovery", post(complete_recovery))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(current_operator))
        .route("/api/v1/auth/session", get(session_status))
        .route("/api/v1/auth/activity", post(record_activity))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(CaState { access, runtime })
}

async fn start_login(
    State(state): State<CaState>,
    Json(request): Json<CredentialsRequest>,
) -> Result<Json<LoginChallengeResponse>, ApiError> {
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || access.start_login(&request.email, &request.password))
            .await?
            .into(),
    ))
}

async fn complete_totp(
    State(state): State<CaState>,
    Extension(log): Extension<AccessAttempt>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || {
                let attempt =
                    access.complete_totp_observed(&request.challenge_token, &request.code);
                log.record_outcome(attempt.user_id, attempt.reason);
                attempt.result
            })
            .await?
            .into(),
    ))
}

async fn complete_recovery(
    State(state): State<CaState>,
    Extension(log): Extension<AccessAttempt>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || {
                let attempt =
                    access.complete_recovery_observed(&request.challenge_token, &request.code);
                log.record_outcome(attempt.user_id, attempt.reason);
                attempt.result
            })
            .await?
            .into(),
    ))
}

async fn logout(State(state): State<CaState>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    let access = state.access;
    state.runtime.run(move || access.logout(&token)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_operator(
    State(state): State<CaState>,
    headers: HeaderMap,
) -> Result<Json<OperatorIdentity>, ApiError> {
    let token = bearer_token(&headers)?;
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || access.authenticate(&token))
            .await?,
    ))
}

async fn session_status(
    State(state): State<CaState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Result<Json<SessionStatusResponse>, ApiError> {
    let token = session_request_token(&headers, query.as_deref(), &body)?;
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || access.session_status(&token))
            .await?
            .into(),
    ))
}

async fn record_activity(
    State(state): State<CaState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Result<Json<SessionStatusResponse>, ApiError> {
    let token = session_request_token(&headers, query.as_deref(), &body)?;
    let access = state.access;
    Ok(Json(
        state
            .runtime
            .run(move || access.record_activity(&token))
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

#[derive(Serialize)]
struct SessionResponse {
    access_token: String,
    token_type: &'static str,
    expires_in_seconds: u64,
    #[serde(flatten)]
    session: SessionStatusResponse,
}

impl From<OperatorSession> for SessionResponse {
    fn from(value: OperatorSession) -> Self {
        Self {
            access_token: value.access_token,
            token_type: "Bearer",
            expires_in_seconds: value.expires_in_seconds,
            session: SessionStatusResponse::new(value.operator, value.session),
        }
    }
}

#[derive(Serialize)]
struct SessionStatusResponse {
    operator: OperatorIdentity,
    policy: SessionPolicyResponse,
    server_now_unix_ms: i64,
    absolute_expires_at_unix_ms: i64,
    idle_expires_at_unix_ms: Option<i64>,
}

impl SessionStatusResponse {
    fn new(operator: OperatorIdentity, session: SessionStatus) -> Self {
        Self {
            operator,
            policy: SessionPolicyResponse {
                absolute_ttl_seconds: session.policy.absolute_ttl_seconds(),
                idle_ttl_seconds: session.policy.idle_ttl_seconds(),
            },
            server_now_unix_ms: session.server_now_unix_ms,
            absolute_expires_at_unix_ms: session.absolute_expires_at_unix_ms,
            idle_expires_at_unix_ms: session.idle_expires_at_unix_ms,
        }
    }
}

impl From<SessionStatus> for SessionStatusResponse {
    fn from(session: SessionStatus) -> Self {
        let operator = OperatorIdentity {
            id: session.principal.id,
            email: session.principal.email.clone(),
        };
        Self::new(operator, session)
    }
}

#[derive(Serialize)]
struct SessionPolicyResponse {
    absolute_ttl_seconds: u64,
    idle_ttl_seconds: Option<u64>,
}
