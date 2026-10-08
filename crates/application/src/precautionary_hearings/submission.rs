use super::{encoding::*, PrecautionaryHearingChange, PrecautionaryHearingCommand};
use crate::{identity::Principal, ApplicationError};
use domain::{cases::CaseId, identity::Role, precautionary_hearings::PrecautionaryHearingValues};

/// PHTXN1 binds an instruction and resolved values; it is not a committed receipt.
/// Cancellation's values must later be verified against the exact stored predecessor.
pub fn precautionary_hearing_submission_bytes(
    actor: &Principal,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    resolved_values: &PrecautionaryHearingValues,
) -> Result<Vec<u8>, ApplicationError> {
    validate_actor_email(&actor.email)?;
    command.result_revision()?;
    if let PrecautionaryHearingChange::Schedule { values, .. }
    | PrecautionaryHearingChange::Replace { values, .. } = &command.change
    {
        if values != resolved_values {
            return Err(ApplicationError::InvalidInput(
                "resolved precautionary values differ".into(),
            ));
        }
    }
    let mut bytes = b"PHTXN1".to_vec();
    bytes.extend_from_slice(actor.id.as_uuid().as_bytes());
    blob(&mut bytes, actor.email.as_bytes());
    bytes.push(match actor.role {
        Role::Owner => 0,
        Role::Litigator => 1,
        Role::Paralegal => 2,
        Role::Client => 3,
    });
    for id in [
        case.as_uuid(),
        command.operation_id.as_uuid(),
        command.hearing_id.as_uuid(),
    ] {
        bytes.extend_from_slice(id.as_bytes());
    }
    bytes.push(command.action().tag());
    bytes.extend_from_slice(&command.expected_revision().to_be_bytes());
    bytes.push(u8::from(command.predecessor().is_some()));
    if let Some(predecessor) = command.predecessor() {
        bytes.extend_from_slice(predecessor.as_bytes());
    }
    bytes.push(u8::from(command.context().is_some()));
    if let Some(context) = command.context() {
        bytes.extend_from_slice(&context.administration_revision.get().to_be_bytes());
        bytes.extend_from_slice(&context.stage_revision.get().to_be_bytes());
        bytes.extend_from_slice(context.context_digest.as_bytes());
    }
    blob(&mut bytes, &resolved_values.canonical_bytes());
    bytes.push(u8::from(command.reason().is_some()));
    if let Some(reason) = command.reason() {
        blob(&mut bytes, reason.as_str().as_bytes());
    }
    Ok(bytes)
}
