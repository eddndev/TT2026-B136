//! Authorized resolution of exact temporal inputs and their observed heads.
mod model;
mod service;
mod sources;
mod validation;

pub use model::*;
pub use service::{DeadlineInputService, PreparedDeadlineInputs};
pub use validation::check_deadline_inputs;

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
