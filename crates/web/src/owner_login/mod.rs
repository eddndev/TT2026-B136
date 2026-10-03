//! Public certificate first factor with mandatory MFA and bounded shared work.

mod input;
mod response;

use std::sync::Arc;

use application::identity::certificate_login::OwnerLoginWorkflow;
use axum::{
    extract::{Request, State},
    routing::post,
    Json, Router,
};

use crate::{dto::LoginChallengeResponse, error::ApiError, runtime::HttpRuntime};

#[derive(Clone)]
struct LoginState {
    workflow: Arc<dyn OwnerLoginWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn OwnerLoginWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/auth/certificate-login/start", post(start))
        .route("/api/v1/auth/certificate-login/proof", post(proof))
        .with_state(LoginState { workflow, runtime })
}

async fn start(
    State(state): State<LoginState>,
    request: Request,
) -> Result<Json<response::StartResponse>, ApiError> {
    let (owner, binding) = input::read::<input::Start>(request, input::START_LIMIT)
        .await?
        .identities()?;
    let challenge = state
        .runtime
        .run(move || state.workflow.start_certificate_login(owner, binding))
        .await?;
    Ok(Json(challenge.into()))
}

async fn proof(
    State(state): State<LoginState>,
    request: Request,
) -> Result<Json<LoginChallengeResponse>, ApiError> {
    let (token, signature) = input::read::<input::Proof>(request, input::PROOF_LIMIT)
        .await?
        .material()?;
    let challenge = state
        .runtime
        .run(move || state.workflow.prove_certificate_login(&token, &signature))
        .await?;
    Ok(Json(challenge.into()))
}
