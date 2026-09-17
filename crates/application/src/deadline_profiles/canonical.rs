use super::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
pub fn deadline_profile_definition_digest(
    hasher: &dyn DocumentHasher,
    definition: &DeadlineProfileDefinition,
) -> Sha256Digest {
    hasher.hash_bytes(&deadline_profile_definition_bytes(definition))
}
/// DPTX1 binds actor, operation, root, action, exact base, algorithm, definition and reason.
pub fn deadline_profile_submission_bytes(
    actor: UserId,
    command: &DeadlineProfileCommand,
    algorithm: DeadlineProfileAlgorithm,
    definition_digest: Sha256Digest,
) -> Vec<u8> {
    encode(
        actor,
        command.profile_id,
        &DeadlineProfileReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            submission_digest: definition_digest,
        },
        algorithm,
        definition_digest,
        command.reason(),
    )
}
pub(super) fn history_submission_bytes(entry: &DeadlineProfileHistoryEntry) -> Vec<u8> {
    encode(
        entry.recorded_by.id,
        entry.id,
        &entry.receipt,
        entry.algorithm,
        entry.definition_digest,
        entry.reason.as_ref(),
    )
}
fn encode(
    actor: UserId,
    id: DeadlineProfileId,
    receipt: &DeadlineProfileReceipt,
    algorithm: DeadlineProfileAlgorithm,
    definition_digest: Sha256Digest,
    reason: Option<&domain::procedural_facts::FactText>,
) -> Vec<u8> {
    let mut bytes = b"DPTX1".to_vec();
    bytes.extend_from_slice(actor.as_uuid().as_bytes());
    bytes.extend_from_slice(receipt.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(id.as_uuid().as_bytes());
    bytes.push(receipt.action.tag());
    bytes.extend_from_slice(&receipt.expected_revision.to_be_bytes());
    bytes.push(algorithm.tag());
    bytes.extend_from_slice(definition_digest.as_bytes());
    bytes.push(u8::from(reason.is_some()));
    if let Some(reason) = reason {
        bytes.extend_from_slice(&(reason.as_str().len() as u32).to_be_bytes());
        bytes.extend_from_slice(reason.as_str().as_bytes());
    }
    bytes
}
pub fn deadline_profile_submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    command: &DeadlineProfileCommand,
    algorithm: DeadlineProfileAlgorithm,
    definition_digest: Sha256Digest,
) -> Sha256Digest {
    hasher.hash_bytes(&deadline_profile_submission_bytes(
        actor,
        command,
        algorithm,
        definition_digest,
    ))
}
