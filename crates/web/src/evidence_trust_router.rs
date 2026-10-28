//! Standalone evidence trust route behind the shared HTTP protection.
use std::sync::Arc;

use application::evidence_trust::EvidenceTrustWorkflow;
use axum::Router;

use crate::runtime::{protect, HttpLimits, HttpRuntime};

/// Builds the read-only evidence trust route with its own default limits.
pub fn evidence_trust_router(workflow: Arc<dyn EvidenceTrustWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        crate::evidence_trust::router(workflow, runtime.clone()),
        runtime,
    )
}
