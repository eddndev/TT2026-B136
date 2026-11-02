//! Bilateral contract routes over the shared bounded HTTP runtime: the case
//! routes of Owner and assigned staff, and the request routes that a
//! designated party reaches by relation. No route looks an account up by
//! e-mail. Rationale: `docs/adr/0086-bilateral-contract-signing.md`.

#[path = "contracts_input.rs"]
mod input;
#[path = "contracts_requests.rs"]
mod requests;
#[path = "contracts_response.rs"]
mod response;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::{
    contracts::{ContractService, ContractWorkflowError},
    ApplicationError,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use domain::cases::CaseId;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

/// The contract service shared by every contract route.
#[derive(Clone)]
pub struct ContractHttp {
    pub service: Arc<ContractService>,
}

#[derive(Clone)]
struct ContractState {
    service: Arc<ContractService>,
    runtime: HttpRuntime,
}

const CASE: &str = "/api/v1/cases/:case/contracts";

pub(crate) fn router(components: ContractHttp, runtime: HttpRuntime) -> Router {
    let state = ContractState {
        service: components.service,
        runtime,
    };
    Router::new()
        .route(CASE, get(list).post(create))
        .route(
            &format!("{CASE}/operations/:operation"),
            get(contract_receipt),
        )
        .route(&format!("{CASE}/:contract"), get(detail))
        .route(&format!("{CASE}/:contract/requests"), post(create_request))
        .route(&format!("{CASE}/:contract/abandon"), post(abandon))
        .layer(DefaultBodyLimit::max(input::BODY_LIMIT))
        .with_state(state.clone())
        .merge(requests::router(state))
}

fn token(request: &Request) -> Result<Zeroizing<String>, ApiError> {
    let token = Zeroizing::new(bearer_token(request.headers())?);
    if request.uri().query().is_some() {
        return Err(input::invalid());
    }
    Ok(token)
}

fn case(raw: &str) -> Result<CaseId, ApiError> {
    Ok(CaseId::from_uuid(input::uuid(raw)?))
}

async fn list(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.case_contracts(&token, case))
        .await?;
    Ok(Json(response::case_contracts(row)?))
}

async fn create(
    State(state): State<ContractState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    let (operation, document, title) = input::read::<input::Creation>(request).await?.values()?;
    let row = state
        .runtime
        .run(move || {
            state
                .service
                .create(&token, case, operation, document, &title)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(response::receipt(&row)?)))
}

async fn detail(
    State(state): State<ContractState>,
    Path((raw, contract)): Path<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    let contract = input::uuid(&contract)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.contract(&token, case, contract))
        .await?;
    Ok(Json(response::contract(&row)?))
}

async fn create_request(
    State(state): State<ContractState>,
    Path((raw, contract)): Path<(String, String)>,
    request: Request,
) -> Result<Response, ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    let contract = input::uuid(&contract)?;
    let command = input::read::<input::Requested>(request).await?.values()?;
    let row = state
        .runtime
        .run(move || {
            match state
                .service
                .create_request(&token, case, contract, &command)
            {
                Err(ApplicationError::Contract(ContractWorkflowError::PartyIneligible(
                    blockers,
                ))) => Ok(Err(blockers)),
                other => other.map(Ok),
            }
        })
        .await?;
    match row {
        Ok(receipt) => {
            Ok((StatusCode::CREATED, Json(response::receipt(&receipt)?)).into_response())
        }
        Err(blockers) => Ok(crate::error::contract::ineligible(blockers)),
    }
}

async fn abandon(
    State(state): State<ContractState>,
    Path((raw, contract)): Path<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    let contract = input::uuid(&contract)?;
    let (operation, expected, reason) =
        input::read::<input::Abandonment>(request).await?.values()?;
    let row = state
        .runtime
        .run(move || {
            state
                .service
                .abandon(&token, case, contract, operation, expected, &reason)
        })
        .await?;
    Ok(Json(response::receipt(&row)?))
}

async fn contract_receipt(
    State(state): State<ContractState>,
    Path((raw, operation)): Path<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case) = (token(&request)?, case(&raw)?);
    let operation: Uuid = input::uuid(&operation)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.contract_receipt(&token, case, operation))
        .await?;
    Ok(Json(response::receipt(&row)?))
}
