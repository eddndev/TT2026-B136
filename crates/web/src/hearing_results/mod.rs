//! Authorized HTTP access to declared hearing sessions and exact historical evidence.
mod mutations;
mod object;
mod projection;
mod query;
mod reads;
mod request;
mod response;
mod sources;
mod time;
mod values;
use crate::{error::ApiError, runtime::HttpRuntime};
use application::hearing_results::{HearingResultId, HearingResultRevision, HearingResultWorkflow};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::{cases::CaseId, hearings::HearingId};
use std::sync::Arc;
#[derive(Clone)]
struct ResultState {
    workflow: Arc<dyn HearingResultWorkflow>,
    runtime: HttpRuntime,
}
#[derive(Clone, Copy)]
struct Scope {
    case: CaseId,
    hearing: HearingId,
}
impl Scope {
    fn parse(case: &str, hearing: &str) -> Result<Self, ApiError> {
        Ok(Self {
            case: CaseId::from_uuid(request::parse_uuid(case, "invalid_case_id")?),
            hearing: HearingId::from_uuid(request::parse_uuid(hearing, "invalid_hearing_id")?),
        })
    }
}
pub(crate) fn router(workflow: Arc<dyn HearingResultWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results/prepare",
            post(mutations::prepare),
        )
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results",
            get(reads::list).post(mutations::create),
        )
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results/:id",
            get(reads::detail).put(mutations::correct),
        )
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results/:id/withdrawal",
            post(mutations::withdraw),
        )
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results/:id/revisions/:revision",
            get(reads::exact),
        )
        .route(
            "/api/v1/cases/:case/hearings/:hearing/results/:id/history",
            get(reads::history),
        )
        .layer(DefaultBodyLimit::max(512 * 1024))
        .with_state(ResultState { workflow, runtime })
}
fn parse_id(value: &str) -> Result<HearingResultId, ApiError> {
    request::parse_uuid(value, "invalid_hearing_result_id").map(HearingResultId::from_uuid)
}
fn parse_revision(value: &str) -> Result<HearingResultRevision, ApiError> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::invalid_body(
            "invalid_hearing_result_revision",
            "revision must be a positive u32",
        ));
    }
    request::revision(value.parse().map_err(|_| {
        ApiError::invalid_body(
            "invalid_hearing_result_revision",
            "revision must be a positive u32",
        )
    })?)
}
