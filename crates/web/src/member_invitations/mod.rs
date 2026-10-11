//! Owner invitations and public recipient enrollment over bounded HTTP work.

mod error;
mod input;
mod output;
mod recipient;

use std::sync::Arc;

use application::identity::invitations::InvitationWorkflow;
use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::{
    request::bearer_token,
    runtime::{protect, HttpRuntime},
    HttpLimits, HttpWorkBudget,
};
use error::InvitationError;

/// Optional invitation capability, independent of direct account enrollment.
#[derive(Clone)]
pub struct InvitationHttp {
    pub workflow: Arc<dyn InvitationWorkflow>,
}

/// Builds invitation routes with the caller's shared blocking-work budget.
pub fn member_invitation_router(
    components: Option<InvitationHttp>,
    limits: HttpLimits,
    budget: HttpWorkBudget,
) -> Result<Router, application::ApplicationError> {
    let runtime = HttpRuntime::with_budget(limits, budget)?;
    Ok(protect(router(components, runtime.clone()), runtime))
}

#[derive(Clone)]
struct InvitationState {
    components: Option<InvitationHttp>,
    runtime: HttpRuntime,
}

impl InvitationState {
    fn workflow(&self) -> Result<Arc<dyn InvitationWorkflow>, InvitationError> {
        self.components
            .as_ref()
            .map(|components| components.workflow.clone())
            .ok_or(InvitationError::Unavailable)
    }
}

pub(crate) fn router(components: Option<InvitationHttp>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/member-invitations", get(list).post(issue))
        .route("/api/v1/member-invitations/:id", get(detail))
        .route("/api/v1/member-invitations/:id/resend", post(resend))
        .route("/api/v1/auth/invitations/prepare", post(recipient::prepare))
        .route("/api/v1/auth/invitations/finish", post(recipient::finish))
        .route("/api/v1/auth/invitations/receipt", post(recipient::receipt))
        .with_state(InvitationState {
            components,
            runtime,
        })
}

fn bearer(request: &Request) -> Result<Zeroizing<String>, InvitationError> {
    bearer_token(request.headers())
        .map(Zeroizing::new)
        .map_err(Into::into)
}

async fn issue(
    State(state): State<InvitationState>,
    request: Request,
) -> Result<(StatusCode, Json<Value>), InvitationError> {
    let token = bearer(&request)?;
    let input = input::read::<input::Issue>(request).await?.request()?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.issue(&token, input)))
        .await?
        .map_err(InvitationError::owner)?;
    Ok((StatusCode::CREATED, Json(output::summary(value)?)))
}

async fn resend(
    State(state): State<InvitationState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let token = bearer(&request)?;
    let id = input::id(&raw)?;
    let input = input::read::<input::Resend>(request).await?.request()?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.resend(&token, id, input)))
        .await?
        .map_err(InvitationError::owner)?;
    Ok(Json(output::summary(value)?))
}

async fn list(
    State(state): State<InvitationState>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let token = bearer(&request)?;
    let query = Query::<input::List>::try_from_uri(request.uri())
        .map_err(|_| InvitationError::Invalid)?
        .0
        .query()?;
    input::empty(request).await?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.list(&token, query)))
        .await?
        .map_err(InvitationError::owner)?;
    Ok(Json(output::page(value)?))
}

async fn detail(
    State(state): State<InvitationState>,
    Path(raw): Path<String>,
    request: Request,
) -> Result<Json<Value>, InvitationError> {
    let token = bearer(&request)?;
    input::no_query(&request)?;
    let id = input::id(&raw)?;
    input::empty(request).await?;
    let workflow = state.workflow()?;
    let value = state
        .runtime
        .run(move || Ok(workflow.get(&token, id)))
        .await?
        .map_err(InvitationError::owner)?;
    Ok(Json(output::summary(value)?))
}
