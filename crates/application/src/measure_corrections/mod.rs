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
pub use preparation::prepare_measure_administrative_record_with_history;
pub use preparation::prepare_measure_record_correction_with_history;
pub use record_history::resolve_measure_records;
pub(crate) use record_history::{
    checked_record_closure, record_history_bounds, CheckedRecordClosure,
};
pub use record_model::*;

mod judicial_record;
mod judicial_view;
pub use judicial_record::OwnedJudicialMeasure;
pub(crate) use record_view::RecordView;

pub(crate) mod decision_history;
mod record_bounds;
mod record_graph;

pub use capture::{
    measure_administrative_capture_with_decision_history_matches,
    measure_administrative_origin_with_decision_history,
};
pub use preparation::prepare_measure_administrative_record_with_decision_history;
pub use record_history::resolve_measure_records_with_decision_history;

pub(crate) use record_index::HistoryView as RecordHistoryView;

mod dependencies;
mod dependency_model;
mod dependency_report;
pub use dependencies::inspect_measure_administrative_dependencies;
pub use dependency_model::*;

mod workflow_error;
mod workflow_evidence;
mod workflow_model;
mod workflow_port;
mod workflow_prepared;
mod workflow_service;
pub use workflow_error::MeasureAdministrativeError;
pub use workflow_model::*;
pub use workflow_port::*;
pub use workflow_prepared::PreparedMeasureAdministrative;
pub use workflow_service::MeasureAdministrativeService;
