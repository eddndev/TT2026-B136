//! Administrative records retain judicial evidence without creating a new decision.
mod capture;
mod encoding;
mod model;
mod preparation;
mod wire;
pub use capture::{measure_administrative_capture_matches, measure_administrative_origin};
pub use encoding::{
    measure_administrative_capture_bytes, measure_administrative_record_bytes,
    measure_administrative_review_bytes, measure_administrative_submission_bytes,
};
pub use model::*;
pub use preparation::{prepare_measure_record_correction, CheckedMeasureAdministrativeReview};
mod record_history;
mod record_index;
mod record_model;
mod record_view;
pub use capture::{
    measure_administrative_capture_with_history_matches, measure_administrative_origin_with_history,
};
pub use preparation::prepare_measure_record_correction_with_history;
pub use record_history::resolve_measure_records;
pub use record_model::*;
