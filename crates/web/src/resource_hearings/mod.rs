//! Explicit resource hearing submission and authorized historical captures.
mod draft;
mod mutations;
mod query;
mod reads;
mod request;
mod response;
mod values;

use crate::runtime::HttpRuntime;
use application::resource_hearings::{ResourceHearingReadWorkflow, ResourceHearingWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

const MAX_BODY: usize = 64 * 1024;
#[derive(Clone)]
struct HearingState {
    workflow: Arc<dyn ResourceHearingWorkflow>,
    reads: Arc<dyn ResourceHearingReadWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(
    workflow: Arc<dyn ResourceHearingWorkflow>,
    reads: Arc<dyn ResourceHearingReadWorkflow>,
    runtime: HttpRuntime,
) -> Router {
    let base = "/api/v1/cases/:case/procedural-resources/:id/activities/resource-hearings";
    Router::new()
        .route(base, get(reads::list))
        .route(&format!("{base}/prepare"), post(mutations::prepare))
        .route(&format!("{base}/submit"), post(mutations::submit))
        .route(&format!("{base}/:hearing"), get(reads::detail))
        .route(
            &format!("{base}/:hearing/revisions/:revision"),
            get(reads::exact),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(HearingState {
            workflow,
            reads,
            runtime,
        })
}
