//! Public certificate first factor with mandatory MFA and bounded shared work.

mod input;
mod response;

use std::sync::Arc;

use application::identity::certificate_login::OwnerLoginWorkflow;
use axum::{
    body::to_bytes,
    extract::{Request, State},
    routing::{get, post},
    Json, Router,
};

use crate::{dto::LoginChallengeResponse, error::ApiError, runtime::HttpRuntime};

#[derive(Clone)]
struct LoginState {
    workflow: Arc<dyn OwnerLoginWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(
    workflow: Option<Arc<dyn OwnerLoginWorkflow>>,
    runtime: HttpRuntime,
) -> Router {
    let enabled = workflow.is_some();
    let availability = Router::new().route(
        "/api/v1/auth/certificate-login/availability",
        get(move |request: Request| availability(request, enabled)),
    );
    let Some(workflow) = workflow else {
        return availability;
    };
    availability.merge(
        Router::new()
            .route("/api/v1/auth/certificate-login/start", post(start))
            .route("/api/v1/auth/certificate-login/proof", post(proof))
            .with_state(LoginState { workflow, runtime }),
    )
}

#[derive(serde::Serialize)]
struct Availability {
    enabled: bool,
}

async fn availability(request: Request, enabled: bool) -> Result<Json<Availability>, ApiError> {
    let invalid = || {
        ApiError::invalid_body(
            "owner_login_invalid_input",
            "certificate login availability requires an empty body and no query string",
        )
    };
    if request.uri().query().is_some() {
        return Err(invalid());
    }
    to_bytes(request.into_body(), 0)
        .await
        .map_err(|_| invalid())?;
    Ok(Json(Availability { enabled }))
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
