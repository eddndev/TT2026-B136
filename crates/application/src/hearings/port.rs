use super::*;
use crate::documents::{DocumentRecord, StageSupportReadLimits};
use crate::ApplicationError;
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};

/// Captures encrypted support and historical references without committing a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingPreparation {
    pub base: Option<HearingDetail>,
    pub context: HearingCaseContext,
    pub participants: Vec<HearingParticipantSnapshot>,
    pub records: Vec<DocumentRecord>,
}

pub trait HearingStore: Send + Sync {
    fn context(
        &self,
        actor: UserId,
        case_id: CaseId,
        at: OffsetDateTime,
    ) -> Result<HearingCaseContext, ApplicationError>;
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: HearingQuery,
        at: OffsetDateTime,
    ) -> Result<HearingPage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: HearingId,
        revision: Option<HearingRevision>,
        at: OffsetDateTime,
    ) -> Result<HearingDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: HearingId,
        query: HearingHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingHistoryPage, ApplicationError>;
    fn agenda(
        &self,
        actor: UserId,
        query: HearingAgendaQuery,
        at: OffsetDateTime,
    ) -> Result<HearingAgendaPage, ApplicationError>;
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: &HearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingPreparation, ApplicationError>;
    /// Rechecks authorization and prepared snapshots after the shared audit lock.
    /// Captures commit time through the adapter's injected clock after that lock.
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedHearingChange,
    ) -> Result<HearingDetail, ApplicationError>;
}

pub trait HearingWorkflow: Send + Sync {
    fn context(&self, token: &str, case_id: CaseId)
        -> Result<HearingCaseContext, ApplicationError>;
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: HearingQuery,
    ) -> Result<HearingPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: HearingId,
        revision: Option<HearingRevision>,
    ) -> Result<HearingDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: HearingId,
        query: HearingHistoryQuery,
    ) -> Result<HearingHistoryPage, ApplicationError>;
    fn agenda(
        &self,
        token: &str,
        query: HearingAgendaQuery,
    ) -> Result<HearingAgendaPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
    ) -> Result<HearingDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<HearingDetail, ApplicationError>;
}
