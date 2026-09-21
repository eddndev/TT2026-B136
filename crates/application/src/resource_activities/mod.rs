//! Audited exact associations, independent of legal or operational activity state.
mod canonical;
mod command;
mod model;
mod port;
mod preparation;
mod prepared;
mod query;
mod reads;
mod service;
mod sources;
mod validation;
pub use canonical::{resource_activity_capture_bytes, resource_activity_submission_bytes};
pub use command::*;
pub use domain::{
    procedural_resources::{ResourceId, ResourceRevision},
    resource_activities::*,
};
pub use model::*;
pub use port::*;
pub use prepared::PreparedResourceActivityChange;
pub use query::*;
pub use service::ResourceActivityService;
pub use validation::{resource_activity_command_from_detail, resource_activity_receipt_matches};

#[derive(Debug, thiserror::Error)]
pub enum ResourceActivityError {
    #[error("resource activity association not found")]
    NotFound,
    #[error("resource activity association changed")]
    RevisionConflict,
    #[error("resource head changed")]
    ResourceRevisionConflict,
    #[error("resource activity operation differs from its receipt")]
    OperationConflict,
    #[error("resource is organizationally archived")]
    ResourceArchived,
    #[error("resource activity already has the requested state")]
    StateUnchanged,
    #[error("resource activity exact source differs")]
    SourceMismatch,
    #[error("resource activity review changed")]
    SubmissionMismatch,
    #[error("stored resource activity is inconsistent: {0}")]
    StoredInconsistent(String),
}
pub(super) fn inconsistent(message: &str) -> crate::ApplicationError {
    ResourceActivityError::StoredInconsistent(message.into()).into()
}
