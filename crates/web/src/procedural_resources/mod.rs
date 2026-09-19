//! Authorized organizational resources and exact declared act history.
mod context;
mod mutations;
mod object;
mod projection;
mod query;
mod reads;
mod request;
mod sources;
mod values;
use crate::{error::ApiError, runtime::HttpRuntime};
use application::procedural_resources::*;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};
use domain::cases::CaseId;
use std::sync::Arc;
#[derive(Clone)]
struct ResourceState {
    workflow: Arc<dyn ProceduralResourceWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(
    workflow: Arc<dyn ProceduralResourceWorkflow>,
    runtime: HttpRuntime,
) -> Router {
    let base = "/api/v1/cases/:case/procedural-resources";
    Router::new()
        .route(base, get(reads::list).post(mutations::create))
        .route(&format!("{base}/prepare"), post(mutations::prepare))
        .route(
            &format!("{base}/:id"),
            get(reads::detail).put(mutations::correct),
        )
        .route(&format!("{base}/:id/acts"), post(mutations::record_act))
        .route(
            &format!("{base}/:id/acts/:act"),
            put(mutations::correct_act),
        )
        .route(&format!("{base}/:id/archive"), post(mutations::archive))
        .route(
            &format!("{base}/:id/reactivation"),
            post(mutations::reactivate),
        )
        .route(
            &format!("{base}/:id/revisions/:revision"),
            get(reads::exact),
        )
        .route(&format!("{base}/:id/history"), get(reads::history))
        .layer(DefaultBodyLimit::max(512 * 1024))
        .with_state(ResourceState { workflow, runtime })
}
fn case(value: &str) -> Result<CaseId, ApiError> {
    request::uuid(value).map(CaseId::from_uuid)
}
fn id(value: &str) -> Result<ResourceId, ApiError> {
    request::uuid(value).map(ResourceId::from_uuid)
}
fn revision(value: &str) -> Result<ResourceRevision, ApiError> {
    if value.is_empty()
        || value.len() > 10
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(request::invalid(
            "revision must be a positive canonical integer",
        ));
    }
    request::revision(
        value
            .parse()
            .map_err(|_| request::invalid("invalid revision"))?,
    )
}
