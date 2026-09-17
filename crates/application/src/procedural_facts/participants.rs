use super::{FactParticipantSnapshot, FactSourceSelection, ProceduralFactError};
use crate::{
    participants::participant_digest,
    typed_participants::{
        subject_digest, typed_participant_digest, ParticipantDetail, ParticipantOverview,
        ParticipantRevisionSnapshot,
    },
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, procedural_facts::FactParticipantRef};

/// Public directory fields derived from one verified exact historical selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactParticipantProjection {
    pub snapshot: FactParticipantSnapshot,
    pub overview: ParticipantOverview,
}

/// Checks the supplied exact material, never directory heads. Archived revisions
/// remain valid sources. This does not authorize access, verify credential
/// signatures, or admit documentary evidence referenced by the historical values.
pub fn resolve_fact_participants(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selection: &FactSourceSelection,
    material: &[ParticipantDetail],
) -> Result<Vec<FactParticipantProjection>, ApplicationError> {
    let expected = selection.participants();
    if expected.len() > 4 || material.len() > 4 || material.len() != expected.len() {
        return Err(inconsistent(
            "participant material count differs from exact selection",
        ));
    }
    let mut ordered = material.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|detail| (detail.id().as_uuid(), detail.revision_number().get()));
    if ordered.windows(2).any(|pair| {
        pair[0].id() == pair[1].id() && pair[0].revision_number() == pair[1].revision_number()
    }) {
        return Err(inconsistent(
            "participant material repeats an exact revision",
        ));
    }
    ordered
        .into_iter()
        .zip(expected)
        .map(|(detail, reference)| {
            if detail.case_id() != case_id
                || detail.id() != reference.id
                || detail.revision_number() != reference.revision
            {
                return Err(inconsistent(
                    "participant material scope or exact reference differs",
                ));
            }
            project(hasher, case_id, *reference, detail)
        })
        .collect()
}

fn project(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    reference: FactParticipantRef,
    detail: &ParticipantDetail,
) -> Result<FactParticipantProjection, ApplicationError> {
    let (overview, values_digest) = match &detail.revision {
        ParticipantRevisionSnapshot::Manual(snapshot) => {
            if detail.bound_subject.is_some() {
                return Err(inconsistent(
                    "manual participant has an unexpected bound subject",
                ));
            }
            let digest = participant_digest(hasher, &snapshot.values);
            if digest != snapshot.values_digest {
                return Err(inconsistent(
                    "manual participant digest differs from its values",
                ));
            }
            (ParticipantOverview::from(snapshot.as_ref()), digest)
        }
        ParticipantRevisionSnapshot::Typed(snapshot) => {
            let digest = typed_participant_digest(hasher, &snapshot.values);
            if digest != snapshot.values_digest {
                return Err(inconsistent(
                    "typed participant digest differs from its values",
                ));
            }
            let subject = detail
                .bound_subject
                .as_ref()
                .ok_or_else(|| inconsistent("typed participant lacks its exact bound subject"))?;
            let bound = snapshot.values.subject();
            if subject.case_id != case_id
                || subject.id != bound.id
                || subject.revision != bound.revision
                || subject.values_digest != bound.values_digest
                || subject_digest(hasher, &subject.values) != subject.values_digest
            {
                return Err(inconsistent(
                    "bound subject scope, revision or digest differs",
                ));
            }
            if !snapshot
                .values
                .kind()
                .accepts_subject(subject.values.kind())
            {
                return Err(inconsistent(
                    "participant kind does not accept its bound subject",
                ));
            }
            let overview = ParticipantOverview {
                case_id,
                id: snapshot.id,
                revision: snapshot.revision,
                display_name: subject.values.display_name().into(),
                procedural_role: snapshot.values.kind().as_str().into(),
                organization: snapshot.values.role().organization().map(str::to_owned),
                directory_status: snapshot.values.directory_status(),
                kind: Some(snapshot.values.kind()),
                subject: Some(bound),
            };
            (overview, digest)
        }
    };
    Ok(FactParticipantProjection {
        snapshot: FactParticipantSnapshot {
            case_id,
            reference,
            values_digest,
            status: overview.directory_status,
            subject: overview.subject,
        },
        overview,
    })
}

fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
