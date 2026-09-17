use super::*;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};

pub fn hearing_values_digest(hasher: &dyn DocumentHasher, values: &HearingValues) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}

/// HTXN1 binds one normalized command to its actor, case and exact value digest.
pub fn hearing_submission_bytes(
    actor: UserId,
    case_id: CaseId,
    command: &HearingCommand,
    values_digest: Sha256Digest,
) -> Vec<u8> {
    let mut bytes = b"HTXN1".to_vec();
    bytes.extend_from_slice(command.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(actor.as_uuid().as_bytes());
    bytes.extend_from_slice(case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(command.hearing_id.as_uuid().as_bytes());
    bytes.push(command.action().tag());
    bytes.extend_from_slice(&command.expected_revision().to_be_bytes());
    bytes.push(u8::from(command.expected_context().is_some()));
    if let Some(context) = command.expected_context() {
        bytes.extend_from_slice(&context.case_revision.get().to_be_bytes());
        bytes.extend_from_slice(&context.stage_revision.get().to_be_bytes());
    }
    bytes.extend_from_slice(values_digest.as_bytes());
    bytes.push(u8::from(command.reason().is_some()));
    if let Some(reason) = command.reason() {
        bytes.extend_from_slice(&(reason.as_str().len() as u32).to_be_bytes());
        bytes.extend_from_slice(reason.as_str().as_bytes());
    }
    bytes
}

pub fn hearing_submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: &HearingCommand,
    values_digest: Sha256Digest,
) -> Sha256Digest {
    hasher.hash_bytes(&hearing_submission_bytes(
        actor,
        case_id,
        command,
        values_digest,
    ))
}
