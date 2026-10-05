//! Exact precautionary evidence and authorized appointment workflows over storage ports.

mod capture_encoding;
mod capture_model;
mod capture_preparation;
pub(crate) mod capture_validation;
mod command;
mod context;
pub(crate) mod context_encoding;
mod encoding;
mod history;
mod participants;
pub(crate) mod source_encoding;
pub(crate) mod source_inventory;
mod submission;
mod support;

pub use capture_encoding::{
    precautionary_hearing_capture_bytes, precautionary_hearing_review_bytes,
};
pub use capture_model::*;
pub use capture_preparation::{
    prepare_precautionary_hearing_capture, CheckedPrecautionaryHearingReview,
};
pub use capture_validation::{
    precautionary_hearing_receipt_matches, precautionary_hearing_transition_matches,
};
pub use command::*;
pub use context::{PrecautionaryContext, PrecautionaryContextMaterial};
pub use history::{
    precautionary_hearing_history_matches, precautionary_hearing_origin, PrecautionaryHearingOrigin,
};
pub use participants::resolve_precautionary_participants;
pub use submission::precautionary_hearing_submission_bytes;
pub use support::admit_precautionary_support;

pub(crate) mod measure_evidence;
mod measure_history;
pub use measure_evidence::{
    prepare_precautionary_hearing_with_history, PrecautionaryHearingPreparationMaterial,
};
pub use measure_history::{
    precautionary_hearing_history_with_measure_history_matches,
    precautionary_hearing_origin_with_measure_history,
    precautionary_hearing_receipt_with_measure_history_matches,
    precautionary_hearing_transition_with_measure_history_matches,
};

mod context_commitments;

mod workflow_error;
mod workflow_evidence;
mod workflow_model;
mod workflow_port;
mod workflow_prepared;
mod workflow_service;
pub use workflow_error::PrecautionaryHearingError;
pub use workflow_model::*;
pub use workflow_port::*;
pub use workflow_prepared::PreparedPrecautionaryHearing;
pub use workflow_service::PrecautionaryHearingService;

mod read_model;
mod read_port;
mod reads;
pub use read_model::*;
pub use read_port::*;
pub use reads::PrecautionaryHearingReadService;

mod read_inventory;
mod record_evidence;
mod record_history;
pub use record_evidence::{
    prepare_precautionary_hearing_with_record_history,
    PrecautionaryHearingRecordPreparationMaterial,
};
pub use record_history::{
    precautionary_hearing_history_with_record_history_matches,
    precautionary_hearing_origin_with_record_history,
    precautionary_hearing_receipt_with_record_history_matches,
    precautionary_hearing_transition_with_record_history_matches,
};

mod decision_history;
pub use decision_history::{
    precautionary_hearing_history_with_decision_history_matches,
    precautionary_hearing_origin_with_decision_history,
    precautionary_hearing_receipt_with_decision_history_matches,
    precautionary_hearing_transition_with_decision_history_matches,
    prepare_precautionary_hearing_with_decision_history,
    PrecautionaryHearingDecisionPreparationMaterial,
};
