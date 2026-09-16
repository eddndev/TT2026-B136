use super::*;
use postgres::Row;
pub(crate) fn subject(
    tx: &mut impl GenericClient,
    case: CaseId,
    id: CaseSubjectId,
    version: Option<SubjectRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<SubjectSnapshot, ApplicationError> {
    let version = version.map(|r| i64::from(r.get()));
    let row=tx.query_opt("SELECT s.case_id,r.* FROM case_subjects s JOIN case_subject_revisions r ON r.subject_id=s.id WHERE s.case_id=$1 AND s.id=$2 AND ($3::bigint IS NULL OR r.revision=$3) ORDER BY r.revision DESC LIMIT 1",&[&case.as_uuid(),&id.as_uuid(),&version]).map_err(port)?.ok_or(ApplicationError::SubjectNotFound)?;
    decode_subject(&row, hasher)
}
pub(crate) fn decode_subject(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<SubjectSnapshot, ApplicationError> {
    let bytes: Vec<u8> = row
        .try_get("values_canonical")
        .map_err(|_| inconsistent())?;
    let values = crate::typed_participant_codec::subject(
        &bytes,
        &row.try_get("values_view").map_err(|_| inconsistent())?,
    )?;
    let values_digest = digest(row.try_get("values_digest").map_err(|_| inconsistent())?)?;
    if subject_digest(hasher, &values) != values_digest {
        return Err(inconsistent());
    }
    Ok(SubjectSnapshot {
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(|_| inconsistent())?),
        id: CaseSubjectId::from_uuid(row.try_get("subject_id").map_err(|_| inconsistent())?),
        revision: subject_revision(row.try_get("revision").map_err(|_| inconsistent())?)?,
        values,
        values_digest,
        changed_at: timestamp(
            &row.try_get::<_, String>("changed_at")
                .map_err(|_| inconsistent())?,
        )?,
        changed_by: captured_actor(row)?,
    })
}
pub(crate) fn typed_detail(
    tx: &mut impl GenericClient,
    case: CaseId,
    id: ParticipantId,
    version: ParticipantRevision,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantDetail, ApplicationError> {
    let row=tx.query_opt("SELECT p.case_id,r.*,e.statement_digest FROM case_participants p JOIN case_participant_typed_revisions r ON r.participant_id=p.id LEFT JOIN participant_credential_evidence e ON e.participant_id=r.participant_id AND e.revision=r.credential_origin_revision WHERE p.case_id=$1 AND p.id=$2 AND r.revision=$3",&[&case.as_uuid(),&id.as_uuid(),&i64::from(version.get())]).map_err(port)?.ok_or(ApplicationError::ParticipantNotFound)?;
    let snapshot = decode_typed(&row, hasher)?;
    let bound = snapshot.values.subject();
    let subject = subject(tx, case, bound.id, Some(bound.revision), hasher)?;
    if subject.values_digest != bound.values_digest
        || !snapshot
            .values
            .kind()
            .accepts_subject(subject.values.kind())
    {
        return Err(inconsistent());
    }
    Ok(ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(snapshot)),
        bound_subject: Some(subject),
    })
}
pub(crate) fn decode_typed(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<TypedParticipantSnapshot, ApplicationError> {
    let bytes: Vec<u8> = row
        .try_get("values_canonical")
        .map_err(|_| inconsistent())?;
    let values = crate::typed_participant_codec::participant(
        &bytes,
        &row.try_get("values_view").map_err(|_| inconsistent())?,
    )?;
    let values_digest = digest(row.try_get("values_digest").map_err(|_| inconsistent())?)?;
    if typed_participant_digest(hasher, &values) != values_digest {
        return Err(inconsistent());
    }
    let id = ParticipantId::from_uuid(row.try_get("participant_id").map_err(|_| inconsistent())?);
    let origin: Option<i64> = row
        .try_get("credential_origin_revision")
        .map_err(|_| inconsistent())?;
    let credential_origin = origin
        .map(|r| {
            Ok::<_, ApplicationError>(ParticipantCredentialRef {
                participant_id: id,
                participant_revision: revision(r)?,
                statement_digest: digest(
                    row.try_get("statement_digest")
                        .map_err(|_| inconsistent())?,
                )?,
            })
        })
        .transpose()?;
    Ok(TypedParticipantSnapshot {
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(|_| inconsistent())?),
        id,
        revision: revision(row.try_get("revision").map_err(|_| inconsistent())?)?,
        values,
        values_digest,
        changed_at: timestamp(
            &row.try_get::<_, String>("changed_at")
                .map_err(|_| inconsistent())?,
        )?,
        changed_by: captured_actor(row)?,
        credential_origin,
        submission_digest: digest(
            row.try_get("submission_digest")
                .map_err(|_| inconsistent())?,
        )?,
        submission_revision: revision(
            row.try_get("submission_revision")
                .map_err(|_| inconsistent())?,
        )?,
    })
}
pub(super) struct ParticipantHead {
    pub revision: ParticipantRevision,
    pub status: DirectoryStatus,
    pub subject: Option<CaseSubjectId>,
}
pub(super) fn head(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ParticipantId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<ParticipantHead>, ApplicationError> {
    if tx
        .query_opt(
            "SELECT id FROM case_participants WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .is_none()
    {
        return Ok(None);
    }
    let detail = crate::participant_postgres::storage::current(tx, case, id, hasher)?;
    Ok(Some(match detail.revision {
        ParticipantRevisionSnapshot::Manual(s) => ParticipantHead {
            revision: s.revision,
            status: s.values.directory_status(),
            subject: None,
        },
        ParticipantRevisionSnapshot::Typed(s) => ParticipantHead {
            revision: s.revision,
            status: s.values.directory_status(),
            subject: Some(s.values.subject().id),
        },
    }))
}
pub(crate) fn status(
    tx: &mut Transaction<'_>,
    current: &TypedParticipantSnapshot,
    status: DirectoryStatus,
    principal: &Principal,
    at: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantDetail, ApplicationError> {
    let mut snapshot = current.clone();
    snapshot.revision = current
        .revision
        .next()
        .ok_or(ApplicationError::ParticipantRevisionExhausted)?;
    snapshot.values = current.values.with_directory_status(status);
    snapshot.values_digest = typed_participant_digest(hasher, &snapshot.values);
    snapshot.changed_at = at.to_offset(UtcOffset::UTC);
    snapshot.changed_by = actor(principal);
    super::write::typed(tx, &snapshot)?;
    let reference = snapshot.values.subject();
    let bound = subject(
        tx,
        snapshot.case_id,
        reference.id,
        Some(reference.revision),
        hasher,
    )?;
    Ok(ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(snapshot)),
        bound_subject: Some(bound),
    })
}
