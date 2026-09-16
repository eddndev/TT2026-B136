use super::*;
use crate::{
    cases::CurrentCaseAdministration,
    documents::{DocumentRecord, StageSupportReadLimits},
    hearings::HearingDetail,
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, hearings::HearingId,
    identity::UserId,
};
/// Exact encrypted and historical inputs; no reservation or write is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultPreparation {
    pub case_id: CaseId,
    pub base: Option<HearingResultDetail>,
    pub administration: CurrentCaseAdministration,
    pub anchor: HearingDetail,
    pub continuation: Option<HearingResultSnapshot>,
    pub attendees: Vec<HearingResultAttendeeSnapshot>,
    pub records: Vec<DocumentRecord>,
}
pub trait HearingResultStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        hearing_id: HearingId,
        query: HearingResultQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultPage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
        at: OffsetDateTime,
    ) -> Result<HearingResultDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: &HearingResultCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingResultPreparation, ApplicationError>;
    /// After the audit lock, recheck actor, membership, active case, exact sources,
    /// target revision and operation uniqueness. Capture Clock only after that lock.
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedHearingResultChange,
    ) -> Result<HearingResultDetail, ApplicationError>;
}
pub trait HearingResultWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        query: HearingResultQuery,
    ) -> Result<HearingResultPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
    ) -> Result<HearingResultDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
    ) -> Result<HearingResultHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
    ) -> Result<HearingResultDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<HearingResultDetail, ApplicationError>;
}
