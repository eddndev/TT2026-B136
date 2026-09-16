//! Authorized hearing scheduling with immutable history and exact operation receipts.

mod error;
mod model;
mod port;
mod prepared;
mod query;

pub use domain::hearings::*;
pub use error::HearingError;
pub use model::*;
pub use port::{HearingPreparation, HearingStore, HearingWorkflow};
pub use prepared::PreparedHearingChange;
pub use query::{
    HearingAgendaCursor, HearingAgendaQuery, HearingHistoryQuery, HearingQuery, HearingStatusFilter,
};

mod canonical;
mod command;
mod service;
mod service_workflow;
mod validation;
mod validation_receipt;
pub use canonical::{hearing_submission_bytes, hearing_submission_digest, hearing_values_digest};
pub use service::HearingService;
pub use validation_receipt::hearing_receipt_matches;
