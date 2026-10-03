//! Authenticated self-Owner binding evidence, without certificate login.

mod input;
mod response;

use std::sync::Arc;

use application::identity::owner_certificates::{OwnerCertificateError, OwnerCertificateService};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};

#[derive(Clone)]
struct BindingState {
    service: Arc<OwnerCertificateService>,
    runtime: HttpRuntime,
}

pub(crate) fn router(service: Arc<OwnerCertificateService>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/auth/certificate-bindings/current", get(current))
        .route(
            "/api/v1/auth/certificate-bindings/:binding/prepare",
            post(prepare),
        )
        .route(
            "/api/v1/auth/certificate-bindings/:binding/register",
            post(register),
        )
        .route("/api/v1/auth/certificate-bindings/:binding", get(receipt))
        .route(
            "/api/v1/auth/certificate-bindings/:binding/withdraw",
            post(withdraw),
        )
        .layer(DefaultBodyLimit::max(input::BODY_LIMIT))
        .with_state(BindingState { service, runtime })
}

fn admission(raw: &str, request: &Request) -> Result<(Zeroizing<String>, Uuid), ApiError> {
    let token = Zeroizing::new(bearer_token(request.headers())?);
    let binding = Uuid::parse_str(raw).map_err(|_| input::invalid())?;
    if binding.is_nil() || request.uri().query().is_some() {
        return Err(input::invalid());
    }
    Ok((token, binding))
}

async fn prepare(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    let certificate = input::read::<input::Preparation>(request)
        .await?
        .certificate()?;
    let prepared = state
        .runtime
        .run(move || {
            state
                .service
                .prepare_registration(&token, binding, &certificate)
        })
        .await?;
    Ok(Json(response::preparation(prepared)))
}

async fn register(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    let submission = input::read::<input::Registration>(request)
        .await?
        .submission()?;
    let receipt = state
        .runtime
        .run(move || {
            state
                .service
                .submit_registration(&token, binding, submission)
        })
        .await?;
    Ok(Json(response::receipt(receipt)?))
}

async fn receipt(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    input::empty(request).await?;
    let receipt = state
        .runtime
        .run(move || state.service.receipt(&token, binding))
        .await?
        .ok_or_else(|| {
            ApiError::from(application::ApplicationError::from(
                OwnerCertificateError::NotFound,
            ))
        })?;
    Ok(Json(response::receipt(receipt)?))
}

async fn current(
    State(state): State<BindingState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = Zeroizing::new(bearer_token(request.headers())?);
    if request.uri().query().is_some() {
        return Err(input::invalid());
    }
    input::empty(request).await?;
    let receipt = state
        .runtime
        .run(move || state.service.current_receipt(&token))
        .await?;
    Ok(Json(
        receipt
            .map(response::receipt)
            .transpose()?
            .unwrap_or(Value::Null),
    ))
}

async fn withdraw(
    State(state): State<BindingState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, binding) = admission(&raw, &request)?;
    let input = input::read::<input::Withdrawal>(request).await?;
    let receipt = state
        .runtime
        .run(move || {
            state
                .service
                .withdraw(&token, binding, input.expected_revision)
        })
        .await?;
    Ok(Json(response::receipt(receipt)?))
}
