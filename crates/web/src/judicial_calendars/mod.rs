//! Global staff calendar references, immutable revisions and civil classification.
mod mutations;
mod object;
mod projection;
mod query;
mod reads;
mod request;
mod response;
mod values;
use crate::{error::ApiError, runtime::HttpRuntime};
use application::judicial_calendars::*;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
const MAX_BODY_BYTES: usize = 1024 * 1024;
#[derive(Clone)]
struct CalendarState {
    workflow: Arc<dyn JudicialCalendarWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn JudicialCalendarWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/judicial-calendars/prepare",
            post(mutations::prepare),
        )
        .route(
            "/api/v1/judicial-calendars",
            get(reads::list).post(mutations::publish),
        )
        .route(
            "/api/v1/judicial-calendars/:id",
            get(reads::detail).put(mutations::replace),
        )
        .route(
            "/api/v1/judicial-calendars/:id/retirement",
            post(mutations::retire),
        )
        .route(
            "/api/v1/judicial-calendars/:id/history",
            get(reads::history),
        )
        .route(
            "/api/v1/judicial-calendars/:id/revisions/:revision",
            get(reads::exact),
        )
        .route(
            "/api/v1/judicial-calendars/:id/revisions/:revision/days",
            get(reads::days),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(CalendarState { workflow, runtime })
}
fn parse_id(value: &str) -> Result<JudicialCalendarId, ApiError> {
    request::parse_uuid(value, "invalid_judicial_calendar_id").map(JudicialCalendarId::from_uuid)
}
fn parse_revision(value: &str) -> Result<JudicialCalendarRevision, ApiError> {
    let number = request::parse_u32(value).ok_or_else(|| {
        ApiError::invalid_body(
            "invalid_judicial_calendar_revision",
            "revision must be a positive u32",
        )
    })?;
    request::revision(number)
}
