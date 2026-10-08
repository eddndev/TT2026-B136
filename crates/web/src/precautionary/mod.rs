//! Exact precautionary appointments and observed case context over authorized ports.
mod administrative;
mod administrative_reads;
mod administrative_request;
mod administrative_router;
mod decision_effects;
mod decision_handlers;
mod decision_reads;
mod decision_request;
mod decision_router;
mod decision_values;
pub use decision_router::measure_decision_router;
pub(crate) use decision_router::router as decision_router;
mod handlers;
mod measure_request;
mod measure_time;
mod record_reads;
pub use administrative_router::measure_administrative_router;
pub(crate) use administrative_router::router as administrative_router;
use administrative_router::AdministrativeState;
mod primitives;
mod projection;
mod reads;
mod request;
mod values;

use crate::runtime::HttpRuntime;
use application::precautionary_hearings::{
    PrecautionaryContextReadWorkflow, PrecautionaryHearingRecordReadWorkflow,
    PrecautionaryHearingRecordWorkflow,
};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::crypto::DocumentHasher;
use std::sync::Arc;

const MAX_BODY: usize = 128 * 1024;
/// Authorized precautionary workflows and the hash port used in context responses.
pub struct PrecautionaryWorkflows {
    pub context: Arc<dyn PrecautionaryContextReadWorkflow>,
    pub hearings: Arc<dyn PrecautionaryHearingRecordWorkflow>,
    pub hearing_reads: Arc<dyn PrecautionaryHearingRecordReadWorkflow>,
    pub administrative: Arc<dyn application::measure_corrections::MeasureAdministrativeWorkflow>,
    pub administrative_reads:
        Arc<dyn application::measure_corrections::MeasureAdministrativeReadWorkflow>,
    pub records: Arc<dyn application::measure_corrections::MeasureRecordReadWorkflow>,
    pub decisions: Arc<dyn application::precautionary_measures::MeasureDecisionRecordWorkflow>,
    pub decision_reads:
        Arc<dyn application::precautionary_measures::MeasureDecisionRecordReadWorkflow>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
}

#[derive(Clone)]
pub(crate) struct HearingState {
    context: Arc<dyn PrecautionaryContextReadWorkflow>,
    workflow: Arc<dyn PrecautionaryHearingRecordWorkflow>,
    reads: Arc<dyn PrecautionaryHearingRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    runtime: HttpRuntime,
}
pub(crate) fn router(
    context: Arc<dyn PrecautionaryContextReadWorkflow>,
    workflow: Arc<dyn PrecautionaryHearingRecordWorkflow>,
    reads: Arc<dyn PrecautionaryHearingRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    runtime: HttpRuntime,
) -> Router {
    let base = "/api/v1/cases/:case/precautionary-hearings";
    Router::new()
        .route(
            "/api/v1/cases/:case/precautionary-context",
            get(handlers::context),
        )
        .route(base, get(reads::list))
        .route(&format!("{base}/prepare"), post(handlers::prepare))
        .route(&format!("{base}/submit"), post(handlers::submit))
        .route(
            &format!("{base}/operations/:operation"),
            get(reads::operation),
        )
        .route(&format!("{base}/:hearing"), get(reads::detail))
        .route(
            &format!("{base}/:hearing/revisions/:revision"),
            get(reads::exact),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(HearingState {
            context,
            workflow,
            reads,
            hasher,
            runtime,
        })
}
