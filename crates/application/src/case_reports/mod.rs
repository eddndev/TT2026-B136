//! Durable operational reports over authorized current case snapshots.
mod canonical;
mod checks;
mod ids;
mod model;
mod port;
mod protection;
mod receipt;
mod service;
mod snapshot;
mod validation;
mod worker;
pub use ids::*;
pub use model::*;
pub use port::*;
pub use protection::*;
pub use service::CaseReportService;
pub use validation::{
    case_report_request_digest, case_report_snapshot_digest, validate_case_report_snapshot,
};
pub use worker::CaseReportWorker;
#[derive(Debug, thiserror::Error)]
pub enum CaseReportError {
    #[error("case report not found")]
    NotFound,
    #[error("case report operation was reused with different content")]
    OperationConflict,
    #[error("case report is not ready")]
    NotReady,
    #[error("case report exceeds its capacity limit")]
    CapacityExceeded,
    #[error("case report access has been revoked")]
    AccessRevoked,
    #[error("case report worker lease was lost")]
    LeaseLost,
    #[error("case report renderer is unavailable")]
    RenderUnavailable,
    #[error("case report rendering failed")]
    RenderFailed,
    #[error("stored case report is inconsistent: {0}")]
    StoredInconsistent(String),
}
