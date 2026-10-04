//! Exact declared measure sources and captures, separate from judicial authority.

mod sources;
mod support;
pub use sources::{resolve_measure_sources, MeasureSourceProjection, MeasureSources};
pub use support::admit_measure_decision_support;
