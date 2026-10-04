//! Exact declared measure sources and captures, separate from judicial authority.

mod decision_capture;
mod decision_encoding;
mod decision_model;
mod decision_preparation;
mod decision_wire;
mod sources;
pub use decision_capture::measure_decision_group_matches;
pub use decision_encoding::{
    measure_capture_bytes, measure_decision_capture_bytes, measure_decision_group_bytes,
    measure_decision_review_bytes, measure_decision_submission_bytes,
};
pub use decision_model::*;
pub use decision_preparation::{prepare_measure_decision_capture, CheckedMeasureDecisionReview};
mod support;
pub use sources::{resolve_measure_sources, MeasureSourceProjection, MeasureSources};
pub use support::admit_measure_decision_support;
