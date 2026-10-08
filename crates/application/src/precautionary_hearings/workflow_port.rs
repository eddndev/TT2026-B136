use super::*;
use crate::{documents::StageSupportReadLimits, identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub trait PrecautionaryHearingStore: Send + Sync {
    /// Reauthorize the full current account and membership before operation lookup.
    /// Replay requires its durable origin and full historical prefix, even on a
    /// closed case. Fresh writes require an active case and exact current head.
    /// New participant selections must be current and active; unchanged exact
    /// prior selections may retain archived records. Preserve bound subjects.
    fn prepare(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &PrecautionaryHearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<PrecautionaryHearingPreparation, ApplicationError>;

    /// Under the shared audit lock recheck the complete principal, membership,
    /// active context, predecessor, source heads, document association and all
    /// prepared material. Do not decrypt or parse while holding the lock.
    /// Write capture, head, origin, operation and audit atomically. An exact raced
    /// operation returns its original full receipt; a collision writes nothing.
    fn commit(
        &self,
        actor: &Principal,
        case_id: CaseId,
        prepared: PreparedPrecautionaryHearing,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
}

pub trait PrecautionaryHearingWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: PrecautionaryHearingCommand,
        confirmation: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
}
