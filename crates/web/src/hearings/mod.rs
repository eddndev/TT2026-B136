//! HTTP adapters for authorized hearing scheduling and agenda queries.
mod mutations;
mod object;
mod projection;
mod query;
mod reads;
mod request;
mod response;
mod time;
mod values;

pub(crate) use projection::overview as agenda_overview;

use crate::{error::ApiError, runtime::HttpRuntime};
use application::hearings::{HearingId, HearingRevision, HearingWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::cases::CaseId;
use std::sync::Arc;

#[derive(Clone)]
struct HearingState {
    workflow: Arc<dyn HearingWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn HearingWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/cases/:case/hearings/context", get(reads::context))
        .route(
            "/api/v1/cases/:case/hearings/prepare",
            post(mutations::prepare),
        )
        .route(
            "/api/v1/cases/:case/hearings",
            get(reads::list).post(mutations::create),
        )
        .route(
            "/api/v1/cases/:case/hearings/:id",
            get(reads::detail).put(mutations::replace),
        )
        .route(
            "/api/v1/cases/:case/hearings/:id/cancellation",
            post(mutations::cancel),
        )
        .route(
            "/api/v1/cases/:case/hearings/:id/revisions/:revision",
            get(reads::exact),
        )
        .route(
            "/api/v1/cases/:case/hearings/:id/history",
            get(reads::history),
        )
        .route("/api/v1/hearings", get(reads::agenda))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(HearingState { workflow, runtime })
}
fn parse_case(value: &str) -> Result<CaseId, ApiError> {
    request::parse_uuid(value, "invalid_case_id").map(CaseId::from_uuid)
}
fn parse_id(value: &str) -> Result<HearingId, ApiError> {
    request::parse_uuid(value, "invalid_hearing_id").map(HearingId::from_uuid)
}
fn parse_revision(value: &str) -> Result<HearingRevision, ApiError> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::invalid_body(
            "invalid_hearing_revision",
            "revision must be a positive u32",
        ));
    }
    request::revision(value.parse().map_err(|_| {
        ApiError::invalid_body(
            "invalid_hearing_revision",
            "revision must be a positive u32",
        )
    })?)
}
#[cfg(test)]
mod route_support;
#[cfg(test)]
mod route_tests;
#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod body_tests;
