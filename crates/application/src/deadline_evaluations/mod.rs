//! Explicit applicability and operational completion of verified temporal inputs.
mod evaluate;
mod model;
pub use evaluate::evaluate_profiled_deadline;
pub use model::*;
