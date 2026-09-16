use super::*;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
pub fn hearing_result_values_digest(
    hasher: &dyn DocumentHasher,
    values: &HearingResultValues,
) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
/// HRTX1 binds the actor, target and both exact historical sources to this command.
pub fn hearing_result_submission_bytes(
    actor: UserId,
    case_id: CaseId,
    command: &HearingResultCommand,
    anchor: &HearingResultAnchor,
    continuation: Option<&HearingResultContinuation>,
    values_digest: Sha256Digest,
) -> Vec<u8> {
    encode_submission(Submission {
        actor,
        case_id,
        operation_id: command.operation_id,
        hearing_id: command.hearing_id,
        result_id: command.result_id,
        action: command.action(),
        expected_revision: command.expected_revision(),
        anchor,
        continuation,
        values_digest,
        reason: command.reason(),
    })
}
struct Submission<'a> {
    actor: UserId,
    case_id: CaseId,
    operation_id: HearingResultOperationId,
    hearing_id: domain::hearings::HearingId,
    result_id: HearingResultId,
    action: HearingResultAction,
    expected_revision: u32,
    anchor: &'a HearingResultAnchor,
    continuation: Option<&'a HearingResultContinuation>,
    values_digest: Sha256Digest,
    reason: Option<&'a HearingResultText>,
}
pub(super) fn history_submission_bytes(entry: &HearingResultHistoryEntry) -> Vec<u8> {
    encode_submission(Submission {
        actor: entry.recorded_by.id,
        case_id: entry.case_id,
        operation_id: entry.receipt.operation_id,
        hearing_id: entry.hearing_id,
        result_id: entry.id,
        action: entry.receipt.action,
        expected_revision: entry.receipt.expected_revision,
        anchor: &entry.anchor,
        continuation: entry.continuation.as_ref(),
        values_digest: entry.values_digest,
        reason: entry.reason.as_ref(),
    })
}
fn encode_submission(input: Submission<'_>) -> Vec<u8> {
    let mut bytes = b"HRTX1".to_vec();
    bytes.extend_from_slice(input.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(input.actor.as_uuid().as_bytes());
    bytes.extend_from_slice(input.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(input.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(input.result_id.as_uuid().as_bytes());
    bytes.push(input.action.tag());
    bytes.extend_from_slice(&input.expected_revision.to_be_bytes());
    bytes.extend_from_slice(&input.anchor.revision.get().to_be_bytes());
    bytes.extend_from_slice(input.anchor.values_digest.as_bytes());
    bytes.extend_from_slice(input.anchor.submission_digest.as_bytes());
    bytes.push(u8::from(input.continuation.is_some()));
    if let Some(previous) = input.continuation {
        bytes.extend_from_slice(previous.hearing_id.as_uuid().as_bytes());
        bytes.extend_from_slice(previous.result_id.as_uuid().as_bytes());
        bytes.extend_from_slice(&previous.revision.get().to_be_bytes());
        bytes.extend_from_slice(previous.values_digest.as_bytes());
        bytes.extend_from_slice(previous.submission_digest.as_bytes());
    }
    bytes.extend_from_slice(input.values_digest.as_bytes());
    bytes.push(u8::from(input.reason.is_some()));
    if let Some(reason) = input.reason {
        bytes.extend_from_slice(&(reason.as_str().len() as u32).to_be_bytes());
        bytes.extend_from_slice(reason.as_str().as_bytes());
    }
    bytes
}

pub fn hearing_result_submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    command: &HearingResultCommand,
    anchor: &HearingResultAnchor,
    continuation: Option<&HearingResultContinuation>,
    values_digest: Sha256Digest,
) -> Sha256Digest {
    hasher.hash_bytes(&hearing_result_submission_bytes(
        actor,
        case_id,
        command,
        anchor,
        continuation,
        values_digest,
    ))
}
