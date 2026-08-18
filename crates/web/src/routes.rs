//! Axum routes that adapt HTTP requests to the document workflow port.

use std::sync::Arc;

use application::documents::DocumentWorkflow;
use application::identity::IdentityWorkflow;
use application::ApplicationError;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::crypto::DocumentId;
use domain::identity::{Permission, Role};
use std::str::FromStr;
use uuid::Uuid;

use crate::dto::{
    AuditResponse, ChallengeCodeRequest, CreateUserRequest, CredentialsRequest, DocumentResponse,
    EnrollmentResponse, LoginChallengeResponse, PrincipalResponse, SessionResponse,
    VerificationResponse,
};
use crate::error::ApiError;

const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const DOCUMENT_NAME_HEADER: &str = "x-document-name";

#[derive(Clone)]
struct AppState {
    workflow: Arc<dyn DocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
}

pub fn router(workflow: Arc<dyn DocumentWorkflow>, identity: Arc<dyn IdentityWorkflow>) -> Router {
    Router::new()
        .route("/api/v1/auth/bootstrap", post(bootstrap_owner))
        .route("/api/v1/auth/login", post(start_login))
        .route("/api/v1/auth/mfa/totp", post(complete_totp))
        .route("/api/v1/auth/mfa/recovery", post(complete_recovery))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(current_user))
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/documents", post(upload_document))
        .route("/api/v1/documents/:id/seal", post(seal_document))
        .route("/api/v1/documents/:id/verify", post(verify_document))
        .route("/api/v1/documents/:id/evidence", get(export_evidence))
        .route("/api/v1/audit/verify", get(verify_audit))
        .layer(DefaultBodyLimit::max(MAX_DOCUMENT_BYTES))
        .with_state(AppState { workflow, identity })
}

async fn bootstrap_owner(
    State(state): State<AppState>,
    Json(request): Json<CredentialsRequest>,
) -> Result<(StatusCode, Json<EnrollmentResponse>), ApiError> {
    let identity = state.identity.clone();
    let result =
        blocking(move || identity.bootstrap_owner(&request.email, &request.password)).await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<EnrollmentResponse>), ApiError> {
    let actor = authorize(&state, &headers, Permission::CreateUser).await?;
    let role = Role::from_str(&request.role)
        .map_err(|_| ApplicationError::InvalidInput("invalid role".to_string()))?;
    let identity = state.identity.clone();
    let result =
        blocking(move || identity.create_user(&actor, &request.email, &request.password, role))
            .await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

async fn start_login(
    State(state): State<AppState>,
    Json(request): Json<CredentialsRequest>,
) -> Result<Json<LoginChallengeResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        blocking(move || identity.start_login(&request.email, &request.password))
            .await?
            .into(),
    ))
}

async fn complete_totp(
    State(state): State<AppState>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        blocking(move || identity.complete_totp(&request.challenge_token, &request.code))
            .await?
            .into(),
    ))
}

async fn complete_recovery(
    State(state): State<AppState>,
    Json(request): Json<ChallengeCodeRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let identity = state.identity.clone();
    Ok(Json(
        blocking(move || identity.complete_recovery(&request.challenge_token, &request.code))
            .await?
            .into(),
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?.to_string();
    let identity = state.identity.clone();
    blocking(move || identity.logout(&token)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PrincipalResponse>, ApiError> {
    let token = bearer_token(&headers)?.to_string();
    let identity = state.identity.clone();
    Ok(Json(
        blocking(move || identity.authenticate(&token))
            .await?
            .into(),
    ))
}

async fn upload_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<DocumentResponse>), ApiError> {
    let actor = authorize(&state, &headers, Permission::CreateDocument).await?;
    let name = required_header(&headers, DOCUMENT_NAME_HEADER)?.to_string();
    let workflow = state.workflow.clone();
    let summary = blocking(move || workflow.upload(&actor.email, &name, &body)).await?;
    Ok((StatusCode::CREATED, Json(summary.into())))
}

async fn seal_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DocumentResponse>, ApiError> {
    let actor = authorize(&state, &headers, Permission::SealDocument).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let summary = blocking(move || workflow.seal(&actor.email, id)).await?;
    Ok(Json(summary.into()))
}

async fn verify_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<VerificationResponse>, ApiError> {
    let actor = authorize(&state, &headers, Permission::VerifyDocument).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let report = blocking(move || workflow.verify(&actor.email, id)).await?;
    Ok(Json(report.into()))
}

async fn export_evidence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let actor = authorize(&state, &headers, Permission::ExportEvidence).await?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let export = blocking(move || workflow.export_evidence(&actor.email, id)).await?;
    let disposition = format!("attachment; filename=\"{}\"", export.file_name);
    let mut response = Response::new(Body::from(export.archive));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).map_err(|_| ApiError::invalid_response_header())?,
    );
    response.headers_mut().insert(
        "x-document-digest",
        HeaderValue::from_str(&export.document_digest_hex)
            .map_err(|_| ApiError::invalid_response_header())?,
    );
    Ok(response)
}

async fn verify_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuditResponse>, ApiError> {
    authorize(&state, &headers, Permission::VerifyAudit).await?;
    let workflow = state.workflow.clone();
    Ok(Json(
        blocking(move || workflow.verify_audit()).await?.into(),
    ))
}

async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<application::identity::Principal, ApiError> {
    let token = bearer_token(headers)?.to_string();
    let identity = state.identity.clone();
    blocking(move || identity.authorize(&token, permission)).await
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or(ApplicationError::InvalidSession.into())
}

fn required_header<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, ApiError> {
    let value = headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::invalid_header(name))?;
    Ok(value)
}

fn parse_id(value: &str) -> Result<DocumentId, ApiError> {
    Uuid::parse_str(value)
        .map(DocumentId::from_uuid)
        .map_err(|_| ApiError::invalid_document_id())
}

async fn blocking<T, F>(task: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ApplicationError> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(Into::into)
}
