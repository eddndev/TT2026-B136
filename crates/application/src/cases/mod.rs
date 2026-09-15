//! Authenticated case metadata and membership use cases.
//!
//! Policy and storage boundaries are recorded in
//! docs/adr/0014-case-membership-and-isolation.md.

mod action;
mod model;
mod port;
mod query;
mod service;

use domain::cases::CaseId;
use domain::identity::UserId;
use serde::{Deserialize, Serialize};

pub use action::CaseAdministrationAction;
pub use domain::case_administration::{
    CaseAdministrationValues, CaseAdministrativeStatus, CaseEditableValues, CaseRevision,
    CaseStageRevision, InitialCaseStage, PenalCaseCreation, PenalCaseProfile,
};
pub use model::{
    case_administration_digest, CaseActorSnapshot, CaseAdministrationDetail,
    CaseAdministrationHistoryPage, CaseAdministrationOverview, CaseAdministrationPage,
    CaseAdministrationSnapshot, CaseInitialStageRegistration, CaseOrigin, CasePenalIdentifiers,
    CurrentCaseAdministration,
};
pub use port::{CaseRepository, CaseWorkflow};
pub use query::{
    CaseAdministrationHistoryQuery, CaseAdministrationQuery, CaseProfileFilter,
    CaseRevisionExpectation, CaseStatusFilter,
};
pub use service::CaseService;

/// Persisted case metadata, independent of document encryption records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseRecord {
    pub id: CaseId,
    pub title: String,
    pub reference: String,
    pub created_by: UserId,
}
