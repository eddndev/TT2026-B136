//! Routes of one signature request, reached by relation: the designated
//! Client and firm signer, and staff with scope on the case.

use super::{input, response, token, ContractState};
use crate::error::ApiError;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

const PREFIX: &str = "/api/v1/contract-requests";

pub(super) fn router(state: ContractState) -> Router {
    Router::new()
        .route(PREFIX, get(mine))
        .route(&format!("{PREFIX}/:request"), get(detail))
        .route(
            &format!("{PREFIX}/:request/content"),
            get(content).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .route(&format!("{PREFIX}/:request/prepare"), post(prepare))
        .route(&format!("{PREFIX}/:request/sign"), post(sign))
        .route(&format!("{PREFIX}/:request/close"), post(close))
        .route(
            &format!("{PREFIX}/:request/operations/:operation"),
            get(receipt),
        )
        .route(
            &format!("{PREFIX}/:request/evidence/:stage"),
            get(evidence).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .layer(DefaultBodyLimit::max(input::BODY_LIMIT))
        .with_state(state)
}

async fn mine(
    State(state): State<ContractState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    input::empty(request).await?;
    let rows = state
        .runtime
        .run(move || state.service.my_requests(&token))
        .await?;
    Ok(Json(response::my_requests(&rows)?))
}

async fn detail(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.request(&token, id))
        .await?;
    Ok(Json(response::request_detail(&row)?))
}

async fn content(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Response, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    input::empty(request).await?;
    let permit = state.runtime.reserve_content()?;
    let service = state.service;
    let (row, permit) = state
        .runtime
        .run(move || Ok((service.content(&token, id)?, permit)))
        .await?;
    crate::document_content::attachment(row, permit)
}

async fn prepare(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    let operation = input::read::<input::Preparation>(request)
        .await?
        .operation()?;
    let row = state
        .runtime
        .run(move || state.service.prepare(&token, id, operation))
        .await?;
    Ok(Json(response::prepared(&row)))
}

async fn sign(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    let submitted = input::read::<input::Signing>(request).await?.values()?;
    let row = state
        .runtime
        .run(move || state.service.sign(&token, id, &submitted))
        .await?;
    Ok(Json(response::receipt(&row)?))
}

async fn close(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    let value = input::read::<input::Closing>(request).await?.values()?;
    let row = state
        .runtime
        .run(move || {
            state.service.close(
                &token,
                id,
                value.operation,
                value.expected_revision,
                value.outcome,
                value.reason.as_deref(),
            )
        })
        .await?;
    Ok(Json(response::receipt(&row)?))
}

async fn receipt(
    State(state): State<ContractState>,
    Path((raw, operation)): Path<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    let operation = input::uuid(&operation)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.request_receipt(&token, id, operation))
        .await?;
    Ok(Json(response::receipt(&row)?))
}

async fn evidence(
    State(state): State<ContractState>,
    Path((raw, stage)): Path<(String, String)>,
    request: Request,
) -> Result<Response, ApiError> {
    let token = token(&request)?;
    let id = input::uuid(&raw)?;
    let stage_value = input::stage(&stage)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.export(&token, id, stage_value))
        .await?;
    if row.request != id || row.stage != stage_value {
        return Err(ApiError::internal());
    }
    let disposition = format!("attachment; filename=\"contrato-{id}-{stage}.zip\"");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).map_err(|_| ApiError::invalid_response_header())?,
        )
        .header("x-content-type-options", "nosniff")
        .body(Body::from(row.archive))
        .map_err(|_| ApiError::internal())
}
