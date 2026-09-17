//! Explicit applicability and operational completion of verified temporal inputs.
mod encoding;
mod evaluate;
mod input_canonical;
mod model;
pub use evaluate::evaluate_profiled_deadline;
pub use model::*;

pub use input_canonical::{deadline_evaluation_input_bytes, decode_deadline_evaluation_input};

mod record;
pub use record::*;
