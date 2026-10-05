use super::{decision_handlers, decision_reads};
use crate::runtime::{protect, HttpRuntime};
use application::precautionary_measures::{
    MeasureDecisionRecordReadWorkflow, MeasureDecisionRecordWorkflow,
};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::crypto::DocumentHasher;
use std::sync::Arc;

pub(super) const MAX_DECISION_BODY: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(super) struct DecisionState {
    pub workflow: Arc<dyn MeasureDecisionRecordWorkflow>,
    pub reads: Arc<dyn MeasureDecisionRecordReadWorkflow>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub runtime: HttpRuntime,
}

/// Builds declared measure decision commands and exact authorized reads.
pub fn measure_decision_router(
    workflow: Arc<dyn MeasureDecisionRecordWorkflow>,
    reads: Arc<dyn MeasureDecisionRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
) -> Router {
    let runtime = HttpRuntime::new(crate::HttpLimits::default());
    protect(router(workflow, reads, hasher, runtime.clone()), runtime)
}

pub(crate) fn router(
    workflow: Arc<dyn MeasureDecisionRecordWorkflow>,
    reads: Arc<dyn MeasureDecisionRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    runtime: HttpRuntime,
) -> Router {
    let base = "/api/v1/cases/:case/measure-decisions";
    Router::new()
        .route(base, get(decision_reads::list))
        .route(&format!("{base}/prepare"), post(decision_handlers::prepare))
        .route(&format!("{base}/submit"), post(decision_handlers::submit))
        .route(
            &format!("{base}/operations/:operation"),
            get(decision_reads::operation),
        )
        .route(&format!("{base}/:decision"), get(decision_reads::detail))
        .layer(DefaultBodyLimit::max(MAX_DECISION_BODY))
        .with_state(DecisionState {
            workflow,
            reads,
            hasher,
            runtime,
        })
}
