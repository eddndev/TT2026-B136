//! Declared hearing sessions with exact historical sources and audited receipts.
mod admission;
mod canonical;
mod command;
mod error;
mod model;
mod port;
mod prepared;
mod query;
pub use canonical::{
    hearing_result_submission_bytes, hearing_result_submission_digest, hearing_result_values_digest,
};
pub use domain::hearing_results::*;
pub use error::HearingResultError;
pub use model::*;
pub use port::{HearingResultPreparation, HearingResultStore, HearingResultWorkflow};
pub use prepared::PreparedHearingResultChange;
pub use query::{HearingResultHistoryQuery, HearingResultQuery, HearingResultStatusFilter};

mod projection;
mod validation_receipt;
pub use validation_receipt::{
    hearing_result_history_receipt_matches, hearing_result_receipt_matches,
    hearing_result_snapshot_receipt_matches,
};

mod draft_validation;
mod service;
mod service_workflow;
mod validation;
pub(crate) use admission::{draft_from_prepared, HearingResultAdmission};
pub(crate) use draft_validation::validate_record_draft;
pub(crate) use service::support_error;
pub use service::HearingResultService;
