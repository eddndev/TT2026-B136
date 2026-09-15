//! Shared HTTP admission and blocking-work limits.
//!
//! See docs/adr/0015-backend-concurrency-and-invariants.md.

use std::{num::NonZeroUsize, sync::Arc};

use application::ApplicationError;
use axum::extract::{Request, State};
use axum::http::{header::CACHE_CONTROL, HeaderValue};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use tokio::sync::Semaphore;

use crate::error::ApiError;

/// Per-server bounds shared by identity, document, case and participant routes.
#[derive(Debug, Clone, Copy)]
pub struct HttpLimits {
    pub max_requests: NonZeroUsize,
    pub max_blocking_operations: NonZeroUsize,
}

impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            max_requests: NonZeroUsize::new(8).expect("the default request limit is positive"),
            max_blocking_operations: NonZeroUsize::new(2)
                .expect("the default worker limit is positive"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct HttpRuntime {
    request_slots: Arc<Semaphore>,
    blocking_slots: Arc<Semaphore>,
}

impl HttpRuntime {
    pub(crate) fn new(limits: HttpLimits) -> Self {
        Self {
            request_slots: Arc::new(Semaphore::new(limits.max_requests.get())),
            blocking_slots: Arc::new(Semaphore::new(limits.max_blocking_operations.get())),
        }
    }

    pub(crate) async fn run<T, F>(&self, task: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, ApplicationError> + Send + 'static,
    {
        let permit = self
            .blocking_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| ApiError::busy())?;
        tokio::task::spawn_blocking(move || {
            // The worker owns its permit even if the HTTP future is dropped.
            let _permit = permit;
            task()
        })
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(Into::into)
    }
}

pub(crate) fn protect(router: Router, runtime: HttpRuntime) -> Router {
    router.layer(middleware::from_fn_with_state(runtime, admit))
}

async fn admit(State(runtime): State<HttpRuntime>, request: Request, next: Next) -> Response {
    let mut response = match runtime.request_slots.try_acquire_owned() {
        Ok(_permit) => next.run(request).await,
        Err(_) => ApiError::busy().into_response(),
    };
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests;
