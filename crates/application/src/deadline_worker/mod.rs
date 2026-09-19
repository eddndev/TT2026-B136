//! Durable technical deadline job consumption and verified historical results.
//!
//! These projections reuse existing reevaluation, receipt and observation
//! models. Implementations preserve human history and audit each completion or
//! deferred attempt without creating a user identity or accepting qualification.

mod evidence;
mod model;
mod port;

pub use evidence::administration_evidence_digest;
pub use model::*;
pub use port::DeadlineWorkerStore;
