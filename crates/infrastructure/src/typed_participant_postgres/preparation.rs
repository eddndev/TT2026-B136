use super::*;
use application::documents::DocumentRecord;
pub(super) fn participant(
    tx: &mut Transaction<'_>,
    case: CaseId,
    request: &ParticipantPreparationRequest,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
    fingerprint: Option<Sha256Digest>,
) -> Result<TypedParticipantPreparation, ApplicationError> {
    let p = &request.proposal;
    let current = storage::head(tx, case, p.participant_id(), hasher)?;
    match (p.expected_participant(), current) {
        (ParticipantExpectation::Absent, None) => {
            if tx
                .query_opt(
                    "SELECT id FROM case_participants WHERE id=$1",
                    &[&p.participant_id().as_uuid()],
                )
                .map_err(port)?
                .is_some()
            {
                return Err(ApplicationError::ParticipantRevisionConflict);
            }
        }
        (ParticipantExpectation::Revision(expected), Some(head)) => {
            if head.revision != expected || head.status != p.values().directory_status() {
                return Err(ApplicationError::ParticipantRevisionConflict);
            }
            if head.subject.is_some_and(|id| id != p.subject().id()) {
                return Err(ApplicationError::ParticipantSubjectChangeForbidden);
            }
        }
        _ => return Err(ApplicationError::ParticipantRevisionConflict),
    }
    let values = match p.subject() {
        SubjectChange::Keep(reference) => {
            let observed = storage::subject(tx, case, reference.id, None, hasher)?;
            if observed.revision != reference.revision
                || observed.values_digest != reference.values_digest
            {
                return Err(ApplicationError::SubjectRevisionConflict);
            }
            observed.values
        }
        SubjectChange::Append {
            id,
            expected,
            values,
        } => {
            match expected {
                SubjectExpectation::Absent => {
                    if tx
                        .query_opt("SELECT id FROM case_subjects WHERE id=$1", &[&id.as_uuid()])
                        .map_err(port)?
                        .is_some()
                    {
                        return Err(ApplicationError::SubjectRevisionConflict);
                    }
                }
                SubjectExpectation::Revision(expected) => {
                    let observed = storage::subject(tx, case, *id, None, hasher)?;
                    if observed.revision != *expected {
                        return Err(ApplicationError::SubjectRevisionConflict);
                    }
                    if observed.values.kind() != values.kind() {
                        return Err(ApplicationError::ParticipantSubjectChangeForbidden);
                    }
                }
            }
            values.clone()
        }
    };
    if subject_digest(hasher, &values) != p.values().subject().values_digest {
        return Err(ApplicationError::SubjectRevisionConflict);
    }
    if !p.values().kind().accepts_subject(values.kind()) {
        return Err(ApplicationError::ParticipantSubjectChangeForbidden);
    }
    if tx.query_opt("SELECT 1 FROM case_participant_typed_revisions r WHERE r.subject_id=$1 AND r.role_kind=$2 AND r.participant_id<>$3 AND r.revision=(SELECT MAX(h.revision) FROM case_participant_typed_revisions h WHERE h.participant_id=r.participant_id)",&[&p.subject().id().as_uuid(),&p.values().kind().as_str(),&p.participant_id().as_uuid()]).map_err(port)?.is_some(){return Err(ApplicationError::ParticipantRoleConflict)}
    let stamp = directory::stamp(tx, case, hasher)?;
    let candidates = candidates::find(
        tx,
        case,
        &values,
        Some(p.subject().id()),
        Some(p.participant_id()),
        fingerprint,
    )?;
    candidates::require_review(&request.review, &stamp, &candidates)?;
    let mut supports = vec![values.identity_support().clone()];
    supports.extend(p.values().role().supports().into_iter().cloned());
    supports.extend(request.review.different.iter().map(|d| d.support.clone()));
    let records = records(tx, case, supports, limits)?;
    let trust = if request.certificate.is_some() {
        Some(
            crate::credential_trust_postgres::current(tx)?
                .ok_or(ApplicationError::CredentialTrustUnavailable)?,
        )
    } else {
        None
    };
    Ok(TypedParticipantPreparation {
        directory_stamp: stamp,
        subject_values: values,
        records,
        trust,
    })
}
pub(super) fn subject(
    tx: &mut Transaction<'_>,
    case: CaseId,
    request: &SubjectReplacementRequest,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<SubjectPreparation, ApplicationError> {
    let current = storage::subject(tx, case, request.id, None, hasher)?;
    if current.revision != request.expected_revision {
        return Err(ApplicationError::SubjectRevisionConflict);
    }
    if current.values.kind() != request.values.kind() {
        return Err(ApplicationError::ParticipantSubjectChangeForbidden);
    }
    let stamp = directory::stamp(tx, case, hasher)?;
    let candidates = candidates::find(tx, case, &request.values, Some(request.id), None, None)?;
    candidates::require_review(&request.review, &stamp, &candidates)?;
    let mut supports = vec![request.values.identity_support().clone()];
    supports.extend(request.review.different.iter().map(|d| d.support.clone()));
    Ok(SubjectPreparation {
        directory_stamp: stamp,
        records: records(tx, case, supports, limits)?,
    })
}
fn records(
    tx: &mut Transaction<'_>,
    case: CaseId,
    mut supports: Vec<ParticipantEvidenceLocator>,
    limits: &StageSupportReadLimits,
) -> Result<Vec<DocumentRecord>, ApplicationError> {
    supports.sort_by_key(|s| (s.reference().id.as_uuid(), s.reference().version.get()));
    if supports
        .windows(2)
        .any(|p| p[0].reference() == p[1].reference() && p[0].digest() != p[1].digest())
    {
        return Err(ApplicationError::ParticipantSupportChanged);
    }
    supports.dedup_by_key(|s| s.reference());
    if supports.len() > 2 {
        return Err(ApplicationError::InvalidInput(
            "participant support batch exceeds two document versions".into(),
        ));
    }
    documents::require_scope(tx, case, &supports)?;
    supports
        .iter()
        .map(|s| documents::load(tx, case, s, limits))
        .collect()
}
