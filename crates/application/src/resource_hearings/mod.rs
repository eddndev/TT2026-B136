//! Review resource-specific appointments against exact declared sources.

mod canonical;
mod model;
mod preparation;
mod service;

pub use model::*;
pub use service::{ResourceHearingService, ResourceHearingStore};
