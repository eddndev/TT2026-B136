use crate::{
    participants::ParticipantActorSnapshot,
    procedural_facts::{resolve_fact_participants, FactParticipantProjection, FactSourceSelection},
    typed_participants::{
        subject_digest, ParticipantDetail, ParticipantRevisionSnapshot, SubjectOverview,
        SubjectSnapshot,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::DocumentHasher,
    precautionary_measures::{MeasureSupervision, MeasureValues},
    procedural_facts::FactParticipantRef,
};

/// Complete historical selections, separate from current authorization and source admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureSources {
    pub subject: SubjectSnapshot,
    pub supervisor: Option<ParticipantDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureSourceProjection {
    pub subject: SubjectOverview,
    pub supervisor: Option<FactParticipantProjection>,
}

/// Resolves historical subject and supervisor labels without asserting judicial authority.
/// This verifies supplied material, not current heads, signatures or document admission.
pub fn resolve_measure_sources(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    values: &MeasureValues,
    sources: &MeasureSources,
) -> Result<MeasureSourceProjection, ApplicationError> {
    let subject = &sources.subject;
    let expected = values.subject();
    if subject.case_id != case_id
        || subject.id != expected.id
        || subject.revision != expected.revision
        || subject.values_digest != expected.values_digest
        || subject_digest(hasher, &subject.values) != subject.values_digest
    {
        return Err(invalid("subject differs from exact selection"));
    }
    provenance(subject.changed_at, &subject.changed_by)?;
    let supervisor = match (values.supervision(), &sources.supervisor) {
        (MeasureSupervision::Unknown { .. }, None) => None,
        (MeasureSupervision::Known { participant, .. }, Some(detail)) => {
            let selection = FactSourceSelection::select_participant(FactParticipantRef {
                id: participant.id(),
                revision: participant.revision(),
            });
            let projection = resolve_fact_participants(
                hasher,
                case_id,
                &selection,
                std::slice::from_ref(detail),
            )?;
            match &detail.revision {
                ParticipantRevisionSnapshot::Manual(value) => {
                    provenance(value.changed_at, &value.changed_by)?;
                }
                ParticipantRevisionSnapshot::Typed(value) => {
                    provenance(value.changed_at, &value.changed_by)?;
                }
            }
            if let Some(bound) = &detail.bound_subject {
                provenance(bound.changed_at, &bound.changed_by)?;
                if bound.case_id == subject.case_id
                    && bound.id == subject.id
                    && bound.revision == subject.revision
                    && bound != subject
                {
                    return Err(invalid("one subject revision has contradictory sources"));
                }
            }
            Some(
                projection
                    .into_iter()
                    .next()
                    .ok_or_else(|| invalid("supervisor was not resolved"))?,
            )
        }
        _ => return Err(invalid("supervisor material differs from declaration")),
    };
    Ok(MeasureSourceProjection {
        subject: SubjectOverview {
            case_id,
            id: subject.id,
            revision: subject.revision,
            kind: subject.values.kind(),
            display_name: subject.values.display_name().into(),
        },
        supervisor,
    })
}

fn provenance(
    at: OffsetDateTime,
    actor: &ParticipantActorSnapshot,
) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("source provenance must use supported UTC"));
    }
    let email = &actor.email;
    if email.is_empty()
        || email.trim() != email
        || email.chars().any(char::is_control)
        || u32::try_from(email.len()).is_err()
    {
        return Err(invalid("invalid captured source actor"));
    }
    Ok(())
}

fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(format!("inconsistent measure source: {message}"))
}
