//! Authorized personal alerts, preferences and durable delivery boundaries.

mod delivery;
mod ids;
mod model;
mod port;
mod preferences;
mod query;
mod service;
mod validation;

pub use delivery::*;
pub use ids::*;
pub use model::*;
pub use port::*;
pub use preferences::*;
pub use query::*;
pub use service::AlertService;

#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("invalid alert input: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Timing(#[from] domain::alerts::AlertTimingError),
    #[error("alert not found")]
    NotFound,
    #[error("alert preferences changed; refresh before saving")]
    RevisionConflict,
    #[error("alert operation was already used with different content")]
    OperationConflict,
    #[error("stored alert is inconsistent: {0}")]
    Stored(String),
}
