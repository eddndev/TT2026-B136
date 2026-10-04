use super::*;
use crate::{documents::StageSupportReadLimits, hearing_results::HearingResultPreparation};
use domain::{cases::CaseId, identity::UserId};

/// Authorized database inputs before document decryption or format admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineInputs {
    pub actor: Principal,
    pub result: HearingResultPreparation,
    pub profile: DeadlineProfileDetail,
    pub profile_head: DeadlineProfileDetail,
    pub calendar: Option<JudicialCalendarDetail>,
    pub calendar_head: Option<JudicialCalendarDetail>,
    pub responsible: DeadlineResponsibleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HearingDerivedDeadlinePreparation {
    Ready(Box<HearingDerivedDeadlineInputs>),
    Replay(Box<HearingDerivedDeadlineRecord>),
}

/// Existing evidence is returned without fabricating a fresh calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HearingDerivedDeadlineReview {
    Ready(Box<HearingDerivedDeadlineDraft>),
    Replay(Box<HearingDerivedDeadlineRecord>),
}

pub trait HearingDerivedDeadlineStore: Send + Sync {
    /// Check current authorization before resolving either fresh inputs or an
    /// exact compound origin. Ordinary component writes are not compound replay.
    /// Release database locks before the application validates support bytes.
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingDerivedDeadlineCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingDerivedDeadlinePreparation, ApplicationError>;

    /// Reauthorize under the audit lock, resolve the reviewed dependencies and
    /// compare admitted encrypted support. Capture the clock once after locking,
    /// then commit result, emitted event, deadline, origin and audit atomically.
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingDerivedDeadline,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError>;
}

pub trait HearingDerivedDeadlineWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
    ) -> Result<HearingDerivedDeadlineReview, ApplicationError>;

    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
        expected_review_digest: Sha256Digest,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError>;
}
