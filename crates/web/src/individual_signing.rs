//! Public individual signing routes over the shared bounded HTTP runtime.

#[path = "individual_signing_documents.rs"]
mod documents;
#[path = "individual_signing_input.rs"]
mod input;
#[path = "individual_signing_response.rs"]
mod response;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::document_signing::{
    DocumentSigningService, SigningBindingService, SigningWorkflowError,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

/// Public proof services; private signing material is never an HTTP dependency.
#[derive(Clone)]
pub struct IndividualSigningHttp {
    pub bindings: Arc<SigningBindingService>,
    pub documents: Arc<DocumentSigningService>,
}
#[derive(Clone)]
struct BindingState {
    service: Arc<SigningBindingService>,
    runtime: HttpRuntime,
}

pub(crate) fn router(components: IndividualSigningHttp, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/identity/signing-certificates/current",
            get(current),
        )
        .route(
            "/api/v1/identity/signing-certificates/:binding/prepare",
            post(prepare),
        )
        .route(
            "/api/v1/identity/signing-certificates/:binding/register",
            post(register),
        )
        .route(
            "/api/v1/identity/signing-certificates/:binding",
            get(receipt),
        )
        .route(
            "/api/v1/identity/signing-certificates/:binding/withdraw",
            post(withdraw),
        )
        .layer(DefaultBodyLimit::max(input::BODY_LIMIT))
        .with_state(BindingState {
            service: components.bindings,
            runtime: runtime.clone(),
        })
        .merge(documents::router(components.documents, runtime))
}
fn token(request: &Request) -> Result<Zeroizing<String>, ApiError> {
    let token = Zeroizing::new(bearer_token(request.headers())?);
    if request.uri().query().is_some() {
        return Err(input::invalid());
    }
    Ok(token)
}
fn admission(raw: &str, request: &Request) -> Result<(Zeroizing<String>, Uuid), ApiError> {
    Ok((token(request)?, input::uuid(raw)?))
}
fn conflict() -> ApiError {
    application::ApplicationError::from(SigningWorkflowError::Conflict).into()
}

async fn prepare(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    let certificate = input::read::<input::BindingPreparation>(request)
        .await?
        .certificate()?;
    let row = state
        .runtime
        .run(move || state.service.prepare(&token, binding, &certificate))
        .await?;
    Ok(Json(response::binding_preparation(row)))
}
async fn register(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    let value = input::read::<input::Registration>(request)
        .await?
        .submission()?;
    if value.statement.material().binding() != binding {
        return Err(conflict());
    }
    let row = state
        .runtime
        .run(move || {
            state.service.register(
                &token,
                &value.statement,
                &value.certificate,
                &value.signature,
            )
        })
        .await?;
    Ok(Json(response::binding_receipt(row)?))
}
async fn receipt(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.get(&token, binding))
        .await?;
    Ok(Json(response::binding_receipt(row)?))
}
async fn current(
    State(state): State<BindingState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.current(&token))
        .await?;
    Ok(Json(
        json!({"binding":row.map(response::binding_receipt).transpose()?}),
    ))
}
async fn withdraw(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.withdraw(&token, binding))
        .await?;
    Ok(Json(response::binding_receipt(row)?))
}
