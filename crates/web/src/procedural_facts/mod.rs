//! Authorized declarations of resolutions and notification practices.
mod mutations;
mod object;
mod projection;
mod query;
mod reads;
mod request;
#[cfg(test)]
mod request_tests;
mod response;
#[cfg(test)]
mod response_tests;
mod scope;
mod sources;
pub(crate) mod values;

use crate::runtime::HttpRuntime;
use application::procedural_facts::{FactFamily, ProceduralFactWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use scope::{RoutePath, Scope};
use std::sync::Arc;
const BODY_LIMIT: usize = 512 * 1024;
#[derive(Clone)]
struct FactState {
    workflow: Arc<dyn ProceduralFactWorkflow>,
    runtime: HttpRuntime,
    family: FactFamily,
}
pub(crate) fn router(workflow: Arc<dyn ProceduralFactWorkflow>, runtime: HttpRuntime) -> Router {
    let mut routes = Router::new();
    for (family, base, item) in [
        (
            FactFamily::Resolution,
            "/api/v1/cases/:case/resolutions",
            ":resolution",
        ),
        (
            FactFamily::Notification,
            "/api/v1/cases/:case/resolutions/:resolution/notifications",
            ":notification",
        ),
    ] {
        routes = routes.merge(
            Router::new()
                .route(base, get(reads::list).post(mutations::create))
                .route(&format!("{base}/prepare"), post(mutations::prepare))
                .route(
                    &format!("{base}/{item}"),
                    get(reads::detail).put(mutations::correct),
                )
                .route(
                    &format!("{base}/{item}/withdrawal"),
                    post(mutations::withdraw),
                )
                .route(
                    &format!("{base}/{item}/revisions/:revision"),
                    get(reads::exact),
                )
                .route(&format!("{base}/{item}/history"), get(reads::history))
                .layer(DefaultBodyLimit::max(BODY_LIMIT))
                .with_state(FactState {
                    workflow: workflow.clone(),
                    runtime: runtime.clone(),
                    family,
                }),
        );
    }
    routes
}
