use super::*;
use crate::ApplicationError;
use domain::{
    cases::CaseId, crypto::Sha256Digest, identity::UserId, procedural_resources::ResourceId,
};

pub trait ResourceHearingStore: Send + Sync {
    /// Authorize current case membership before lookup. Resolve an exact prior
    /// creation only from its immutable hearing receipt AND durable origin marker;
    /// an independently created hearing or association never qualifies as replay.
    /// Otherwise read exact historical resource/act captures and the current head
    /// independently. Require current selected participant revisions, bounded to
    /// 32. Support must already be admitted in the selected resource or act.
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceHearingCommand,
    ) -> Result<ResourceHearingPreparation, ApplicationError>;

    /// Under the shared audit lock reauthorize membership, full actor identity,
    /// active case/resource and every reviewed resource and participant head.
    /// Compare the complete prepared material before writing the hearing, its
    /// initial resource association, durable origin and audit in ONE transaction.
    /// A conflicting operation writes nothing. A raced exact operation returns
    /// its original complete creation, including its original timestamp.
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceHearing,
    ) -> Result<ResourceHearingCreation, ApplicationError>;
}

pub trait ResourceHearingWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
    ) -> Result<ResourceHearingDraft, ApplicationError>;

    /// Explicit repetition preserves the command and digest. An existing origin
    /// returns its original creation; absence still follows normal creation rules.
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceHearingCreation, ApplicationError>;
}
