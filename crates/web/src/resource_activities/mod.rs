//! Exact organizational associations to existing case activities.
pub(crate) use projection::{context as validate_context, detail as exact_projection};
pub(crate) use selection::{
    command as project_command, resource as project_resource, Act as ActInput,
    Resource as ResourceInput,
};
pub(crate) use sources::resource_sources;
mod current;
mod mutations;
mod projection;
mod query;
mod reads;
mod request;
mod selection;
mod sources;
mod targets;
use crate::{error::ApiError, runtime::HttpRuntime};
use application::resource_activities::{ResourceActivityId, ResourceActivityWorkflow, ResourceId};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::cases::CaseId;
use std::sync::Arc;
#[derive(Clone)]
struct ResourceActivityState {
    workflow: Arc<dyn ResourceActivityWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn ResourceActivityWorkflow>, runtime: HttpRuntime) -> Router {
    let base = "/api/v1/cases/:case/procedural-resources/:id/activities";
    Router::new()
        .route(
            "/api/v1/cases/:case/hearings/:id/resource-associations",
            get(targets::hearing),
        )
        .route(
            "/api/v1/cases/:case/deadlines/:id/resource-associations",
            get(targets::deadline),
        )
        .route(base, get(reads::list).post(mutations::link))
        .route(&format!("{base}/prepare"), post(mutations::prepare))
        .route(&format!("{base}/:association"), get(reads::detail))
        .route(
            &format!("{base}/:association/unlink"),
            post(mutations::unlink),
        )
        .route(&format!("{base}/:association/history"), get(reads::history))
        .route(
            &format!("{base}/:association/revisions/:revision"),
            get(reads::exact),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(ResourceActivityState { workflow, runtime })
}
fn case(v: &str) -> Result<CaseId, ApiError> {
    request::uuid(v).map(CaseId::from_uuid)
}
fn resource(v: &str) -> Result<ResourceId, ApiError> {
    request::uuid(v).map(ResourceId::from_uuid)
}
fn id(v: &str) -> Result<ResourceActivityId, ApiError> {
    request::uuid(v).map(ResourceActivityId::from_uuid)
}
