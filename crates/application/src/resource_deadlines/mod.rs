//! Explicit deadline registration and exact resource association in one transaction.
mod model;
mod port;
mod preparation;
mod prepared;
mod receipt;
mod service;
pub use model::*;
pub use port::*;
pub use preparation::prepare_resource_deadline_change;
pub use prepared::PreparedResourceDeadline;
pub use receipt::{resource_deadline_result_draft, resource_deadline_submission_digest};
pub use service::ResourceDeadlineService;
fn inconsistent(message: &str) -> crate::ApplicationError {
    crate::resource_activities::ResourceActivityError::StoredInconsistent(message.into()).into()
}
