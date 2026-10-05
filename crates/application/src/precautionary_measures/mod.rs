//! Exact declared measure sources and captures, separate from judicial authority.

mod anchor_encoding;
mod decision_capture;
mod decision_encoding;
mod decision_model;
mod decision_preparation;
pub(crate) mod decision_wire;
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

mod effect_resolution;
mod history_inventory;
mod history_model;
mod history_preparation;
mod history_validation;
pub use history_model::*;
pub use history_preparation::prepare_measure_decision_with_history;
pub use history_validation::{
    measure_decision_group_with_history_matches, measure_group_origin, resolve_measure_targets,
};

pub(crate) use history_inventory::add_sources as add_measure_group_sources;
pub(crate) use history_inventory::shape as measure_group_shape;
pub(crate) use history_validation::resolve_measure_closure;

mod anchor_validation;

mod workflow_error;
mod workflow_evidence;
mod workflow_model;
mod workflow_port;
mod workflow_prepared;
mod workflow_service;
pub use workflow_error::MeasureDecisionError;
pub use workflow_model::*;
pub use workflow_port::*;
pub use workflow_prepared::PreparedMeasureDecision;
pub use workflow_service::MeasureDecisionService;

mod read_inventory;
mod read_model;
mod read_port;
mod reads;
pub use read_model::*;
pub use read_port::*;
pub use reads::MeasureDecisionReadService;

mod record_legacy;
pub(crate) use anchor_validation::{
    selections as record_anchor_selections, shape as record_anchor_shape,
};
pub(crate) use effect_resolution::selections as record_effect_selections;
pub(crate) use history_model::origin as legacy_group_origin;
pub(crate) use record_legacy::validate_legacy_record_node;
mod record_decision_anchor;
mod record_decision_capture;
mod record_decision_effects;
mod record_decision_encoding;
mod record_decision_model;
mod record_decision_preparation;
mod record_decision_wire;
pub use crate::measure_corrections::decision_history::{
    measure_decision_group_v2_matches, measure_group_origin_v2,
    prepare_measure_decision_with_record_history,
};
pub use record_decision_encoding::{
    measure_capture_v2_bytes, measure_decision_group_v2_bytes, measure_decision_review_v2_bytes,
};
pub use record_decision_model::*;
pub(crate) use record_decision_preparation::prepare_record_flat;
pub use record_decision_preparation::CheckedMeasureDecisionReviewV2;

pub(crate) use record_decision_encoding::record_decision_group_shape;

pub(crate) use record_decision_preparation::add_record_decision_sources;

mod record_workflow_evidence;
mod record_workflow_model;
mod record_workflow_port;
mod record_workflow_prepared;
mod record_workflow_service;
pub use record_workflow_model::*;
pub use record_workflow_port::*;
pub use record_workflow_prepared::PreparedMeasureDecisionRecord;
pub use record_workflow_service::MeasureDecisionRecordService;
