use super::{encoding::*, PrecautionaryHearingSources};
use crate::{
    procedural_facts::FactParticipantProjection,
    typed_participants::{ParticipantDetail, ParticipantRevisionSnapshot, SubjectRevisionRef},
    ApplicationError,
};
use domain::{clock::OffsetDateTime, participants::DirectoryStatus};

pub(super) fn sources(
    bytes: &mut Vec<u8>,
    sources: &PrecautionaryHearingSources,
) -> Result<(), ApplicationError> {
    count(bytes, sources.participants.len())?;
    for detail in &sources.participants {
        participant(bytes, detail)?;
    }
    super::context_encoding::document(bytes, &sources.support);
    Ok(())
}

fn participant(bytes: &mut Vec<u8>, detail: &ParticipantDetail) -> Result<(), ApplicationError> {
    bytes.push(match detail.revision {
        ParticipantRevisionSnapshot::Manual(_) => 0,
        ParticipantRevisionSnapshot::Typed(_) => 1,
    });
    bytes.extend_from_slice(detail.case_id().as_uuid().as_bytes());
    bytes.extend_from_slice(detail.id().as_uuid().as_bytes());
    bytes.extend_from_slice(&detail.revision_number().get().to_be_bytes());
    match &detail.revision {
        ParticipantRevisionSnapshot::Manual(source) => {
            blob(bytes, &source.values.canonical_bytes());
            bytes.extend_from_slice(source.values_digest.as_bytes());
            provenance(bytes, source.changed_at, &source.changed_by)?;
        }
        ParticipantRevisionSnapshot::Typed(source) => {
            blob(bytes, &source.values.canonical_bytes());
            bytes.extend_from_slice(source.values_digest.as_bytes());
            provenance(bytes, source.changed_at, &source.changed_by)?;
            bytes.extend_from_slice(&source.submission_revision.get().to_be_bytes());
            bytes.extend_from_slice(source.submission_digest.as_bytes());
            bytes.push(u8::from(source.credential_origin.is_some()));
            if let Some(origin) = &source.credential_origin {
                bytes.extend_from_slice(origin.participant_id.as_uuid().as_bytes());
                bytes.extend_from_slice(&origin.participant_revision.get().to_be_bytes());
                bytes.extend_from_slice(origin.statement_digest.as_bytes());
            }
        }
    }
    bytes.push(u8::from(detail.bound_subject.is_some()));
    if let Some(source) = &detail.bound_subject {
        bytes.extend_from_slice(source.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(source.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&source.revision.get().to_be_bytes());
        blob(bytes, &source.values.canonical_bytes());
        bytes.extend_from_slice(source.values_digest.as_bytes());
        provenance(bytes, source.changed_at, &source.changed_by)?;
    }
    Ok(())
}

pub(super) fn projections(
    bytes: &mut Vec<u8>,
    participants: &[FactParticipantProjection],
) -> Result<(), ApplicationError> {
    count(bytes, participants.len())?;
    for item in participants {
        let source = &item.snapshot;
        bytes.extend_from_slice(source.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(source.reference.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&source.reference.revision.get().to_be_bytes());
        bytes.extend_from_slice(source.values_digest.as_bytes());
        bytes.push(status(source.status));
        subject(bytes, source.subject);
        let view = &item.overview;
        bytes.extend_from_slice(view.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(view.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&view.revision.get().to_be_bytes());
        checked_text(bytes, &view.display_name)?;
        checked_text(bytes, &view.procedural_role)?;
        bytes.push(u8::from(view.organization.is_some()));
        if let Some(organization) = &view.organization {
            checked_text(bytes, organization)?;
        }
        bytes.push(status(view.directory_status));
        bytes.push(u8::from(view.kind.is_some()));
        if let Some(kind) = view.kind {
            bytes.push(kind.tag());
        }
        subject(bytes, view.subject);
    }
    Ok(())
}

fn subject(bytes: &mut Vec<u8>, subject: Option<SubjectRevisionRef>) {
    bytes.push(u8::from(subject.is_some()));
    if let Some(subject) = subject {
        bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
        bytes.extend_from_slice(subject.values_digest.as_bytes());
    }
}

fn status(status: DirectoryStatus) -> u8 {
    match status {
        DirectoryStatus::Active => 0,
        DirectoryStatus::Archived => 1,
    }
}

fn count(bytes: &mut Vec<u8>, count: usize) -> Result<(), ApplicationError> {
    if count > 32 {
        return Err(ApplicationError::InvalidInput(
            "too many precautionary sources".into(),
        ));
    }
    bytes.extend_from_slice(&(count as u32).to_be_bytes());
    Ok(())
}

fn checked_text(bytes: &mut Vec<u8>, text: &str) -> Result<(), ApplicationError> {
    u32::try_from(text.len())
        .map_err(|_| ApplicationError::InvalidInput("source text too long".into()))?;
    blob(bytes, text.as_bytes());
    Ok(())
}

fn provenance(
    bytes: &mut Vec<u8>,
    at: OffsetDateTime,
    actor: &crate::participants::ParticipantActorSnapshot,
) -> Result<(), ApplicationError> {
    validate_actor_email(&actor.email)?;
    super::context_encoding::timestamp(bytes, at);
    bytes.extend_from_slice(actor.id.as_uuid().as_bytes());
    blob(bytes, actor.email.as_bytes());
    Ok(())
}
