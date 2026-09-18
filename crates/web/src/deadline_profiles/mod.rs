//! Explicit global and case profile collections with editable definitions and receipts.
mod definition;
pub(crate) mod definition_projection;
mod expected;
mod mutations;
mod query;
mod reads;
mod request;
mod response;
mod rule;
mod scope;
use crate::{judicial_calendars::object, runtime::HttpRuntime};
use application::deadline_profiles::DeadlineProfileWorkflow;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use scope::RoutePath;
use std::sync::Arc;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone)]
struct ProfileState {
    workflow: Arc<dyn DeadlineProfileWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn DeadlineProfileWorkflow>, runtime: HttpRuntime) -> Router {
    let mut routes = Router::new();
    for base in [
        "/api/v1/deadline-profiles",
        "/api/v1/cases/:case_id/deadline-profiles",
    ] {
        routes = routes
            .route(&format!("{base}/prepare"), post(mutations::prepare))
            .route(base, get(reads::list).post(mutations::publish))
            .route(
                &format!("{base}/:id"),
                get(reads::detail).put(mutations::replace),
            )
            .route(&format!("{base}/:id/retirement"), post(mutations::retire))
            .route(&format!("{base}/:id/history"), get(reads::history))
            .route(
                &format!("{base}/:id/revisions/:revision"),
                get(reads::exact),
            );
    }
    routes
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(ProfileState { workflow, runtime })
}
