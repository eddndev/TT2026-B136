use super::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
pub fn judicial_calendar_values_digest(
    hasher: &dyn DocumentHasher,
    values: &JudicialCalendarValues,
) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
/// JCTX1 binds actor, operation, root, action, exact base, values and reason.
pub fn judicial_calendar_submission_bytes(
    actor: UserId,
    command: &JudicialCalendarCommand,
    values_digest: Sha256Digest,
) -> Vec<u8> {
    encode(
        actor,
        command.calendar_id,
        &JudicialCalendarReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            submission_digest: values_digest,
        },
        values_digest,
        command.reason(),
    )
}
pub(super) fn history_submission_bytes(entry: &JudicialCalendarHistoryEntry) -> Vec<u8> {
    encode(
        entry.recorded_by.id,
        entry.id,
        &entry.receipt,
        entry.values_digest,
        entry.reason.as_ref(),
    )
}
fn encode(
    actor: UserId,
    id: JudicialCalendarId,
    receipt: &JudicialCalendarReceipt,
    values_digest: Sha256Digest,
    reason: Option<&JudicialCalendarReason>,
) -> Vec<u8> {
    let mut bytes = b"JCTX1".to_vec();
    bytes.extend_from_slice(actor.as_uuid().as_bytes());
    bytes.extend_from_slice(receipt.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(id.as_uuid().as_bytes());
    bytes.push(receipt.action.tag());
    bytes.extend_from_slice(&receipt.expected_revision.to_be_bytes());
    bytes.extend_from_slice(values_digest.as_bytes());
    bytes.push(u8::from(reason.is_some()));
    if let Some(reason) = reason {
        bytes.extend_from_slice(&(reason.as_str().len() as u32).to_be_bytes());
        bytes.extend_from_slice(reason.as_str().as_bytes());
    }
    bytes
}
pub fn judicial_calendar_submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    command: &JudicialCalendarCommand,
    values_digest: Sha256Digest,
) -> Sha256Digest {
    hasher.hash_bytes(&judicial_calendar_submission_bytes(
        actor,
        command,
        values_digest,
    ))
}
