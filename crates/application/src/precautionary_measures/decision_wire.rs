use super::*;
use crate::{
    case_stages::StageSupportSnapshot,
    identity::Principal,
    precautionary_hearings::{context_encoding, source_encoding},
    ApplicationError,
};
pub(super) use context_encoding::timestamp;
use domain::{
    identity::Role, precautionary_hearings::PrecautionaryMeasureRef,
    typed_participants::SubjectKind,
};

pub(super) fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(format!("inconsistent measure decision: {message}"))
}

pub(super) fn blob(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), ApplicationError> {
    let size = u32::try_from(value.len()).map_err(|_| invalid("field length overflow"))?;
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

pub(super) fn bounded(count: usize) -> Result<(), ApplicationError> {
    if count > 32 {
        return Err(invalid("group exceeds 32 affected measures"));
    }
    Ok(())
}

pub(super) fn no_anchor(bytes: &mut Vec<u8>, present: bool) -> Result<(), ApplicationError> {
    if present {
        return Err(invalid(
            "linked decision requires complete hearing evidence",
        ));
    }
    bytes.push(0);
    Ok(())
}

pub(super) fn actor(bytes: &mut Vec<u8>, actor: &Principal) -> Result<(), ApplicationError> {
    if actor.email.is_empty()
        || actor.email.trim() != actor.email
        || actor.email.chars().any(char::is_control)
    {
        return Err(invalid("invalid captured actor text"));
    }
    bytes.extend_from_slice(actor.id.as_uuid().as_bytes());
    blob(bytes, actor.email.as_bytes())?;
    bytes.push(match actor.role {
        Role::Owner => 0,
        Role::Litigator => 1,
        Role::Paralegal => 2,
        Role::Client => 3,
    });
    Ok(())
}

pub(super) fn support(
    bytes: &mut Vec<u8>,
    value: &StageSupportSnapshot,
) -> Result<(), ApplicationError> {
    u32::try_from(value.name.len()).map_err(|_| invalid("support name length overflow"))?;
    context_encoding::document(bytes, value);
    Ok(())
}

pub(super) fn result(
    bytes: &mut Vec<u8>,
    value: &ReviewedMeasureResult,
) -> Result<(), ApplicationError> {
    bytes.extend_from_slice(value.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    bytes.extend_from_slice(value.origin.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(value.origin.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(value.effect_key.as_uuid().as_bytes());
    bytes.push(match value.action {
        MeasureCaptureAction::Impose => 0,
        MeasureCaptureAction::Confirm => 1,
        MeasureCaptureAction::Modify => 2,
        MeasureCaptureAction::Revoke => 3,
        MeasureCaptureAction::Cease => 4,
        MeasureCaptureAction::SubstituteOut => 5,
        MeasureCaptureAction::SubstituteIn => 6,
    });
    bytes.push(u8::from(value.previous.is_some()));
    if let Some(previous) = value.previous {
        reference(bytes, previous);
    }
    blob(bytes, &value.values.canonical_bytes())?;
    source_encoding::subject_snapshot(bytes, &value.sources.subject)?;
    bytes.push(u8::from(value.sources.supervisor.is_some()));
    if let Some(supervisor) = &value.sources.supervisor {
        source_encoding::participant(bytes, supervisor)?;
    }
    let view = &value.projection.subject;
    bytes.extend_from_slice(view.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(view.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&view.revision.get().to_be_bytes());
    bytes.push(match view.kind {
        SubjectKind::NaturalPerson => 0,
        SubjectKind::InstitutionalBody => 1,
    });
    blob(bytes, view.display_name.as_bytes())?;
    bytes.push(u8::from(value.projection.supervisor.is_some()));
    if let Some(supervisor) = &value.projection.supervisor {
        source_encoding::projection(bytes, supervisor)?;
    }
    Ok(())
}

fn reference(bytes: &mut Vec<u8>, value: PrecautionaryMeasureRef) {
    bytes.extend_from_slice(value.id().as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision().get().to_be_bytes());
    bytes.extend_from_slice(value.digest().as_bytes());
}
