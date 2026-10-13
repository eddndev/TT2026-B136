//! Bounded authenticated CA commands; no office routes or arbitrary execution.
use std::sync::Arc;

use application::ca_operations::{CaCommand, CaOperations};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde_json::Value;

use crate::{
    error::ApiError,
    request::bearer_token,
    runtime::{protect, HttpLimits, HttpRuntime},
};

#[derive(Clone)]
struct CaState {
    operations: Arc<CaOperations>,
    runtime: HttpRuntime,
}

pub fn ca_operations_router(operations: Arc<CaOperations>, limits: HttpLimits) -> Router {
    let runtime = HttpRuntime::new(limits);
    let router = Router::new()
        .route("/api/v1/ca/operations", post(execute))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .with_state(CaState {
            operations,
            runtime: runtime.clone(),
        });
    protect(router, runtime)
}

async fn execute(
    State(state): State<CaState>,
    headers: HeaderMap,
    Json(command): Json<CaCommand>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .runtime
            .run(move || state.operations.execute(&token, command))
            .await?,
    ))
}
