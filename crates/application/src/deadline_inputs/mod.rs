//! Authorized resolution of exact temporal inputs and their observed heads.
pub(crate) mod encoding;
mod heads;
mod model;
mod service;
mod sources;
mod validation;

pub use encoding::{deadline_input_request_bytes, decode_deadline_input_request};
pub use heads::DeadlineInputHeads;
pub use model::*;
pub use service::{DeadlineInputService, PreparedDeadlineInputs};
pub use validation::{
    check_deadline_inputs, extract_checked_deadline_inputs, CheckedDeadlineTrigger,
};

use crate::ApplicationError;
use domain::identity::UserId;

/// Resolve all selected inputs under current case authorization in one transaction.
pub trait DeadlineInputStore: Send + Sync {
    fn load(
        &self,
        actor: UserId,
        request: &DeadlineInputRequest,
    ) -> Result<DeadlineInputMaterial, ApplicationError>;
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineInputError::Inconsistent(error.to_string()).into()
}
