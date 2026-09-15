use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::identity::UserId;

use super::{
    CaseStageChange, CaseStageDetail, CaseStageExpectation, CaseStagePage, CaseStagePreparation,
    CaseStageQuery, CaseStageRevision, PreparedCaseStageChange, StageAdoption,
    StageSupportReadLimits, StageTransition,
};
use crate::ApplicationError;

pub trait CaseStageWorkflow: Send + Sync {
    fn get(&self, token: &str, case_id: CaseId) -> Result<CaseStageDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        query: CaseStageQuery,
    ) -> Result<CaseStagePage, ApplicationError>;
    fn adopt(
        &self,
        token: &str,
        case_id: CaseId,
        expected: CaseStageExpectation,
        adoption: StageAdoption,
    ) -> Result<CaseStageDetail, ApplicationError>;
    fn transition(
        &self,
        token: &str,
        case_id: CaseId,
        expected: CaseStageRevision,
        transition: StageTransition,
    ) -> Result<CaseStageDetail, ApplicationError>;
}

/// Revalidates actor, role and case membership in each transaction.
pub trait CaseStageStore: Send + Sync {
    /// Commits a read audit event before returning the current stage or absence.
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseStageDetail, ApplicationError>;
    /// Merges initial and changed entries before exclusive descending pagination.
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: CaseStageQuery,
        at: OffsetDateTime,
    ) -> Result<CaseStagePage, ApplicationError>;
    /// Checks exact support scope, active complete profile, expected head and edge.
    /// Bounds ciphertext and evidence before transfer or decoding; releases locks
    /// before cryptographic and structural validation, without a success event.
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        expected: CaseStageExpectation,
        change: &CaseStageChange,
        limits: &StageSupportReadLimits,
    ) -> Result<CaseStagePreparation, ApplicationError>;
    /// Repeats mutable checks and compares the entire prepared document snapshots,
    /// including captured evidence. A concurrent seal requires explicit validation
    /// again. Records the current administrative revision, actor and audit atomically,
    /// returning its committed projection with no subsequent read.
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        expected: CaseStageExpectation,
        prepared: PreparedCaseStageChange,
        at: OffsetDateTime,
    ) -> Result<CaseStageDetail, ApplicationError>;
}
