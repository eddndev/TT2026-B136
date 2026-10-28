//! Read-only trust material for the in-browser evidence validator.
//!
//! Any authenticated principal may read it; it is not under a case because a
//! local evidence file has none. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod response;

use std::sync::Arc;

use application::evidence_trust::EvidenceTrustWorkflow;
use axum::{
    body::to_bytes,
    extract::{Request, State},
    routing::get,
    Json, Router,
};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};

#[derive(Clone)]
struct TrustState {
    workflow: Arc<dyn EvidenceTrustWorkflow>,
    runtime: HttpRuntime,
}

pub(crate) fn router(workflow: Arc<dyn EvidenceTrustWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/evidence/trust", get(current))
        .with_state(TrustState { workflow, runtime })
}

async fn current(
    State(state): State<TrustState>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = Zeroizing::new(bearer_token(request.headers())?);
    if request.uri().query().is_some() {
        return Err(invalid());
    }
    let body = to_bytes(request.into_body(), 0)
        .await
        .map_err(|_| invalid())?;
    if !body.is_empty() {
        return Err(invalid());
    }
    let trust = state
        .runtime
        .run(move || state.workflow.current(&token))
        .await?;
    Ok(Json(response::trust(&trust)?))
}

fn invalid() -> ApiError {
    ApiError::invalid_body(
        "evidence_trust_invalid_input",
        "the evidence trust request takes no query or body",
    )
}
