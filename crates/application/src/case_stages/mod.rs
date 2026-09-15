//! Audited procedural stage changes with exact, validated documentary supports.

mod action;
mod model;
mod port;
mod prepared;
mod query;
mod service;

pub use crate::documents::{StageDocumentFormat, StageFormatPolicy, StageSupportReadLimits};
pub use action::CaseStageAction;
pub use domain::case_stages::*;
pub use model::{
    case_stage_digest, CaseStageDetail, CaseStageEntry, CaseStagePage, CaseStagePreparation,
    CaseStageSnapshot, CurrentCaseStage, StageSupportSnapshot,
};
pub use port::{CaseStageStore, CaseStageWorkflow};
pub use prepared::{PreparedCaseStageChange, ValidatedStageSupport};
pub use query::{CaseStageExpectation, CaseStageQuery};
pub use service::CaseStageService;
