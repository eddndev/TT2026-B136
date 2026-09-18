//! Audited deadline registration and attention over immutable evaluated inputs.
mod canonical;
mod tracked;
mod tracked_state;
pub use tracked::*;
mod command;
pub(crate) mod evidence;
mod evidence_hearing;
mod model;
mod port;
mod preparation;
mod prepared;
mod query;
mod receipt;
mod responsibles;
pub use canonical::{deadline_capture_bytes, deadline_review_bytes, deadline_submission_bytes};
pub use domain::deadlines::{DeadlineId, DeadlineOperationId, DeadlineRevision, DeadlineStatus};
pub use model::*;
pub use port::*;
pub use preparation::prepare_deadline_change;
pub use prepared::*;
pub use query::*;
pub use receipt::{deadline_history_receipt_matches, deadline_receipt_matches};
pub use responsibles::*;

#[derive(Debug, thiserror::Error)]
pub enum DeadlineError {
    #[error("deadline not found")]
    NotFound,
    #[error("deadline revision changed")]
    RevisionConflict,
    #[error("deadline operation already exists")]
    OperationConflict,
    #[error("deadline is retired")]
    Retired,
    #[error("deadline revision is exhausted")]
    RevisionExhausted,
    #[error("select the currently published deadline profile revision")]
    ProfileUnavailable,
    #[error("deadline submission differs from preparation")]
    SubmissionMismatch,
    #[error("selected deadline responsible account is unavailable")]
    ResponsibleUnavailable,
    #[error("stored deadline is inconsistent: {0}")]
    StoredInconsistent(String),
    #[error("invalid deadline field: {0}")]
    Invalid(&'static str),
}
impl From<domain::deadlines::DeadlineValueError> for crate::ApplicationError {
    fn from(error: domain::deadlines::DeadlineValueError) -> Self {
        use domain::deadlines::DeadlineValueError as E;
        match error {
            E::RevisionExhausted => DeadlineError::RevisionExhausted,
            E::InvalidRevision => DeadlineError::Invalid("revision"),
            E::InvalidStatus => DeadlineError::Invalid("status"),
        }
        .into()
    }
}
fn inconsistent(message: &str) -> crate::ApplicationError {
    DeadlineError::StoredInconsistent(message.into()).into()
}

mod service;
mod service_query;
mod service_validation;
pub use service::DeadlineService;

mod successor;
pub use successor::deadline_successor_matches;
pub(crate) use successor::validate_administration_capture;

mod preparation_tracked;
mod profile_selection;
pub use preparation_tracked::prepare_tracked_deadline_change;
