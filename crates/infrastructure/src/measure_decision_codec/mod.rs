//! Strict persisted values; encoding consistency does not establish durable source provenance.
mod outcome;
mod primitives;
pub(crate) mod temporal;
mod values;

use application::ApplicationError;
type Result<T> = std::result::Result<T, ApplicationError>;
pub use outcome::{outcome, outcome_view};
pub use values::{decision_values, decision_view, measure_values, measure_view};
