use super::*;
use crate::{documents::StageSupportReadLimits, identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub trait PrecautionaryHearingRecordStore: Send + Sync {
    /// Reauthorize the current full principal and membership before operation lookup.
    /// Replay retains its original full prefix, even on a closed case. Fresh commands
    /// require an active case and the exact current hearing predecessor. New participant
    /// selections must be current and active; unchanged exact selections may be archived.
    /// Review targets select exact Valid captures without requiring current measure heads.
    fn prepare(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &PrecautionaryHearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<PrecautionaryHearingRecordPreparation, ApplicationError>;

    /// Recheck authorization, context, hearing head, selected sources, encrypted support
    /// and full durable dependency evidence under the shared audit lock. Preserve exact
    /// bound historical subjects. Do not decrypt or parse inside the transaction.
    /// Commit hearing, origin, operation and audit atomically, or return the exact raced
    /// original operation without recapturing its actor, context or timestamp.
    fn commit(
        &self,
        actor: &Principal,
        case_id: CaseId,
        prepared: PreparedPrecautionaryHearingRecord,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError>;
}

pub trait PrecautionaryHearingRecordWorkflow: Send + Sync {
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
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError>;
}
