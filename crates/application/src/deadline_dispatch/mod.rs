//! Bounded durable expansion of dependency events and undeclared deadline history.
//!
//! Dispatch creates stable work identities and advances a persisted cursor. It
//! does not calculate deadlines, accept legal qualifications or execute jobs.

mod model;
mod port;

pub use model::*;
pub use port::DeadlineDispatchStore;
