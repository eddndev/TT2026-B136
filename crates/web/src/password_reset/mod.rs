//! Public reset transport with bounded admission and no session issuance.

mod dto;
mod error;
mod handlers;

use std::sync::Arc;

use application::identity::password_reset::PasswordResetService;
use axum::{routing::post, Router};
use zeroize::Zeroizing;

use crate::runtime::HttpRuntime;

/// Immediate admission disposition, never an account or delivery outcome.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestAdmission {
    Accepted,
    Busy,
    Unavailable,
}

/// The implementation must return immediately without waiting for reset work.
///
/// It owns accepted bounded email input. Busy admission must not queue work;
/// availability describes the whole consumer, never a particular account.
/// Normalization and account lookup belong to the application service.
pub trait PasswordResetRequests: Send + Sync {
    fn try_submit(&self, email: Zeroizing<String>) -> RequestAdmission;
}

/// Request admission and a separate concrete completion service.
#[derive(Clone)]
pub struct PasswordResetHttp {
    pub requests: Arc<dyn PasswordResetRequests>,
    pub completion: Arc<PasswordResetService>,
}

#[derive(Clone)]
struct State {
    components: Option<PasswordResetHttp>,
    runtime: HttpRuntime,
}

pub(crate) fn router(components: Option<PasswordResetHttp>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/auth/password-reset/request",
            post(handlers::request),
        )
        .route(
            "/api/v1/auth/password-reset/complete",
            post(handlers::complete),
        )
        .with_state(State {
            components,
            runtime,
        })
}
