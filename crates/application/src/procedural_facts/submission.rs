use super::*;
use crate::ApplicationError;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};

pub fn fact_values_digest(
    hasher: &dyn DocumentHasher,
    values: &ProceduralFactValues,
) -> Sha256Digest {
    match values {
        ProceduralFactValues::Resolution(values) => hasher.hash_bytes(&values.canonical_bytes()),
        ProceduralFactValues::Notification(values) => hasher.hash_bytes(&values.canonical_bytes()),
    }
}
/// PFTXN1 binds the command and exact admitted sources, without administrative CAS.
pub fn fact_submission_bytes(
    actor: UserId,
    case_id: CaseId,
    command: &ProceduralFactCommand,
    values_digest: Sha256Digest,
    sources_digest: Sha256Digest,
) -> Result<Vec<u8>, ApplicationError> {
    command.result_revision()?;
    Ok(Submission {
        actor,
        case_id,
        operation_id: command.operation_id(),
        target: command.target(),
        action: command.action(),
        expected_revision: command.expected_revision(),
        values_digest,
        sources_digest,
        reason: command.reason(),
    }
    .bytes())
}
pub fn fact_submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: &ProceduralFactCommand,
    values_digest: Sha256Digest,
    sources_digest: Sha256Digest,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&fact_submission_bytes(
        actor,
        case_id,
        command,
        values_digest,
        sources_digest,
    )?))
}
pub(super) struct Submission<'a> {
    pub actor: UserId,
    pub case_id: CaseId,
    pub operation_id: FactOperationId,
    pub target: FactTarget,
    pub action: FactAction,
    pub expected_revision: u32,
    pub values_digest: Sha256Digest,
    pub sources_digest: Sha256Digest,
    pub reason: Option<&'a FactText>,
}
impl Submission<'_> {
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = b"PFTXN1".to_vec();
        bytes.extend_from_slice(self.operation_id.as_uuid().as_bytes());
        bytes.extend_from_slice(self.actor.as_uuid().as_bytes());
        bytes.extend_from_slice(self.case_id.as_uuid().as_bytes());
        match self.target {
            FactTarget::Resolution(id) => {
                bytes.push(0);
                bytes.extend_from_slice(id.as_uuid().as_bytes());
            }
            FactTarget::Notification { id, resolution_id } => {
                bytes.push(1);
                bytes.extend_from_slice(id.as_uuid().as_bytes());
                bytes.extend_from_slice(resolution_id.as_uuid().as_bytes());
            }
        }
        bytes.push(match self.action {
            FactAction::Record => 0,
            FactAction::Correct => 1,
            FactAction::Withdraw => 2,
        });
        bytes.extend_from_slice(&self.expected_revision.to_be_bytes());
        bytes.extend_from_slice(self.values_digest.as_bytes());
        bytes.extend_from_slice(self.sources_digest.as_bytes());
        match self.reason {
            None => bytes.push(0),
            Some(reason) => {
                bytes.push(1);
                bytes.extend_from_slice(&(reason.as_str().len() as u32).to_be_bytes());
                bytes.extend_from_slice(reason.as_str().as_bytes());
            }
        }
        bytes
    }
}
