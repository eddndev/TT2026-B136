//! Case-scoped deadline commands and immutable historical calculations.
mod input;
mod input_projection;
#[cfg(test)]
mod legacy_tests;
mod metadata;
mod mutations;
mod pages;
mod query;
mod reads;
mod request;
mod response;
mod responsibles;
mod result;
mod result_blocks;
#[cfg(test)]
mod result_test_support;
#[cfg(test)]
mod result_tests;
mod result_trace;
mod scope;
mod selection;
mod sources;
use crate::{judicial_calendars::object, runtime::HttpRuntime};
use application::deadlines::DeadlineWorkflow;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use scope::RoutePath;
use std::sync::Arc;
// No profile corpus is embedded in commands; bounded DEVI1 declarations fit this budget.
const MAX_BODY_BYTES: usize = 1024 * 1024;
#[derive(Clone)]
struct DeadlineState {
    workflow: Arc<dyn DeadlineWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn DeadlineWorkflow>, runtime: HttpRuntime) -> Router {
    let base = "/api/v1/cases/:case_id/deadlines";
    Router::new()
        .route(base, get(reads::list).post(mutations::register))
        .route(&format!("{base}/prepare"), post(mutations::prepare))
        .route(&format!("{base}/responsibles"), get(responsibles::list))
        .route(
            &format!("{base}/:id"),
            get(reads::detail).put(mutations::correct),
        )
        .route(&format!("{base}/:id/attention"), post(mutations::attention))
        .route(&format!("{base}/:id/retirement"), post(mutations::retire))
        .route(&format!("{base}/:id/history"), get(reads::history))
        .route(
            &format!("{base}/:id/revisions/:revision"),
            get(reads::exact),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(DeadlineState { workflow, runtime })
}
