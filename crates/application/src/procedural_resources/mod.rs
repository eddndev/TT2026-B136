//! Organizational resources linked to exact historical resolutions and evidence.
mod canonical;
mod command;
mod context;
mod model;
mod port;
mod preparation;
mod prepared;
mod query;
mod reads;
mod service;
mod sources;
mod validation;

pub use canonical::{resource_capture_bytes, resource_sources_bytes, resource_submission_bytes};
pub use command::*;
pub use domain::procedural_resources::*;
pub use model::*;
pub use port::*;
pub use prepared::PreparedResourceChange;
pub use query::*;
pub use service::ProceduralResourceService;
pub use validation::{resource_command_from_detail, resource_receipt_matches};

#[derive(Debug, thiserror::Error)]
pub enum ProceduralResourceError {
    #[error("procedural resource not found")]
    NotFound,
    #[error("procedural resource revision changed")]
    RevisionConflict,
    #[error("procedural resource operation differs from its receipt")]
    OperationConflict,
    #[error("procedural resource is organizationally archived")]
    Archived,
    #[error("procedural resource already has the requested organizational state")]
    StateUnchanged,
    #[error("procedural resource review changed")]
    SubmissionMismatch,
    #[error("stored procedural resource is inconsistent: {0}")]
    StoredInconsistent(String),
}
pub(super) fn inconsistent(message: &str) -> crate::ApplicationError {
    ProceduralResourceError::StoredInconsistent(message.into()).into()
}
