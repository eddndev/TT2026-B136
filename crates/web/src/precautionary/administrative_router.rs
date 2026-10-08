use super::{administrative, administrative_reads, record_reads, MAX_BODY};
use crate::runtime::{protect, HttpRuntime};
use application::measure_corrections::{
    MeasureAdministrativeReadWorkflow, MeasureAdministrativeWorkflow, MeasureRecordReadWorkflow,
};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use domain::crypto::DocumentHasher;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct AdministrativeState {
    pub workflow: Arc<dyn MeasureAdministrativeWorkflow>,
    pub reads: Arc<dyn MeasureAdministrativeReadWorkflow>,
    pub records: Arc<dyn MeasureRecordReadWorkflow>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub runtime: HttpRuntime,
}
/// Builds administrative corrections and exact measure reads over authorized ports.
pub fn measure_administrative_router(
    workflow: Arc<dyn MeasureAdministrativeWorkflow>,
    reads: Arc<dyn MeasureAdministrativeReadWorkflow>,
    records: Arc<dyn MeasureRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
) -> Router {
    let runtime = HttpRuntime::new(crate::HttpLimits::default());
    protect(
        router(workflow, reads, records, hasher, runtime.clone()),
        runtime,
    )
}
pub(crate) fn router(
    workflow: Arc<dyn MeasureAdministrativeWorkflow>,
    reads: Arc<dyn MeasureAdministrativeReadWorkflow>,
    records: Arc<dyn MeasureRecordReadWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    runtime: HttpRuntime,
) -> Router {
    let operations = "/api/v1/cases/:case/measure-administrative-operations";
    let measures = "/api/v1/cases/:case/measures";
    Router::new()
        .route(operations, get(administrative_reads::list))
        .route(
            &format!("{operations}/prepare"),
            post(administrative::prepare),
        )
        .route(
            &format!("{operations}/submit"),
            post(administrative::submit),
        )
        .route(
            &format!("{operations}/:operation"),
            get(administrative_reads::operation),
        )
        .route(measures, get(record_reads::list))
        .route(&format!("{measures}/:measure"), get(record_reads::current))
        .route(
            &format!("{measures}/:measure/revisions/:revision"),
            get(record_reads::exact),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(AdministrativeState {
            workflow,
            reads,
            records,
            hasher,
            runtime,
        })
}
