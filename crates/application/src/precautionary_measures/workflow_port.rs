use super::*;
use crate::{documents::StageSupportReadLimits, identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub trait MeasureDecisionStore: Send + Sync {
    /// Reauthorize the full current principal and membership before operation
    /// lookup. A durable replay remains accessible on a closed case. Fresh work
    /// requires active complete context and exact current affected measure heads.
    /// Resolve every exact subject, participant, document and anchor separately
    /// from current heads. Verify each anchor's durable origin and full hearing
    /// prefix; flat captured material alone cannot establish their existence.
    fn prepare(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &MeasureDecisionCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureDecisionPreparation, ApplicationError>;

    /// Under the shared audit lock recheck the complete principal, membership,
    /// active context, target heads, exact sources, retained anchor origins and
    /// complete reviewed material. Compare the encrypted support without parsing
    /// or decrypting inside the lock. Reject reused identities against the whole
    /// durable inventory, not only the supplied closure. Commit decision, all
    /// measure revisions/heads, substitutions, origin, operation and one group
    /// audit atomically. No-change decisions still create a decision/group. An
    /// exact raced operation returns its original capture; any collision writes nothing.
    fn commit(
        &self,
        actor: &Principal,
        case_id: CaseId,
        prepared: PreparedMeasureDecision,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError>;
}

pub trait MeasureDecisionWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionReview, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        confirmation: MeasureDecisionConfirmation,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError>;
}
