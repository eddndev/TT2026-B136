use super::*;
impl PostgresTypedParticipantStore {
    pub(super) fn subject_list(
        &self,
        actor_id: UserId,
        case: CaseId,
        query: SubjectQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, false)?;
        let after = query.after_id().map(CaseSubjectId::as_uuid);
        let kind = query.kind().map(|k| match k {
            SubjectKind::NaturalPerson => "natural_person",
            SubjectKind::InstitutionalBody => "institutional_body",
        });
        let rows=tx.query("SELECT s.id,r.revision,r.display_name,r.subject_kind,r.changed_at,r.changed_by,r.changed_by_email,
            (r.values_digest=sha256(r.values_canonical) AND r.values_view=typed_subject_values(r.values_canonical)
             AND r.subject_kind=r.values_view->>'kind'
             AND r.display_name=CASE r.subject_kind WHEN 'natural_person' THEN COALESCE(r.values_view->'name'->>'known',r.values_view->'name'->>'label') ELSE r.values_view->>'name' END
             AND r.name_known=(r.subject_kind='institutional_body' OR r.values_view->'name'?'known')
             AND r.declared_identifier IS NOT DISTINCT FROM CASE r.subject_kind WHEN 'natural_person' THEN r.values_view->'curp'->>'known' ELSE r.values_view->'institutional_identifier'->>'known' END
             AND r.identity_document_id=(r.values_view->'identity_support'->>'document_id')::uuid
             AND r.identity_document_version=(r.values_view->'identity_support'->>'version')::bigint
             AND r.identity_document_digest=decode(r.values_view->'identity_support'->>'digest','hex')
             AND r.identity_document_locator=r.values_view->'identity_support'->>'locator'
             AND EXISTS(SELECT 1 FROM users WHERE id=r.changed_by)) AS integrity
            FROM case_subjects s JOIN LATERAL(SELECT * FROM case_subject_revisions WHERE subject_id=s.id ORDER BY revision DESC LIMIT 1) r ON TRUE
            WHERE s.case_id=$1 AND ($2::uuid IS NULL OR s.id>$2) AND ($3::text IS NULL OR strpos(r.display_name COLLATE \"C\",$3)>0)
                AND ($4::text IS NULL OR r.subject_kind=$4) ORDER BY s.id LIMIT $5",&[&case.as_uuid(),&after,&query.name(),&kind,&(i64::from(query.limit())+1)]).map_err(|_|inconsistent())?;
        let has_more = rows.len() > query.limit() as usize;
        let subjects = rows
            .iter()
            .take(query.limit() as usize)
            .map(|row| {
                if !row
                    .try_get::<_, bool>("integrity")
                    .map_err(|_| inconsistent())?
                {
                    return Err(inconsistent());
                }
                timestamp(
                    &row.try_get::<_, String>("changed_at")
                        .map_err(|_| inconsistent())?,
                )?;
                captured_actor(row)?;
                Ok(SubjectOverview {
                    case_id: case,
                    id: CaseSubjectId::from_uuid(row.get(0)),
                    revision: subject_revision(row.get(1))?,
                    display_name: row.get(2),
                    kind: candidates::kind(&row.get::<_, String>(3))?,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_after_id = has_more.then(|| subjects.last().unwrap().id);
        audit(
            &mut tx,
            &principal,
            "participant.subjects_listed",
            &format!("case:{case}:subjects"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(SubjectPage {
            subjects,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn subject_get(
        &self,
        actor_id: UserId,
        case: CaseId,
        id: CaseSubjectId,
        version: Option<SubjectRevision>,
        at: OffsetDateTime,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, false)?;
        let result = storage::subject(&mut tx, case, id, version, self.hasher.as_ref())?;
        audit(
            &mut tx,
            &principal,
            "participant.subject_read",
            &format!(
                "case:{case}:subject:{id}:revision:{}:sha256:{}",
                result.revision.get(),
                result.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    pub(super) fn subject_history_page(
        &self,
        actor_id: UserId,
        case: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<SubjectHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, false)?;
        if tx
            .query_opt(
                "SELECT id FROM case_subjects WHERE id=$1 AND case_id=$2",
                &[&id.as_uuid(), &case.as_uuid()],
            )
            .map_err(port)?
            .is_none()
        {
            return Err(ApplicationError::SubjectNotFound);
        }
        let before = query.before_revision().map(|r| i64::from(r.get()));
        let rows=tx.query("SELECT s.case_id,r.* FROM case_subjects s JOIN case_subject_revisions r ON r.subject_id=s.id WHERE s.id=$1 AND s.case_id=$2 AND ($3::bigint IS NULL OR r.revision<$3) ORDER BY r.revision DESC LIMIT $4",&[&id.as_uuid(),&case.as_uuid(),&before,&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        let revisions = rows
            .iter()
            .take(query.limit() as usize)
            .map(|r| storage::decode_subject(r, self.hasher.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let next_before_revision = has_more.then(|| revisions.last().unwrap().revision);
        audit(
            &mut tx,
            &principal,
            "participant.subject_history_listed",
            &format!("case:{case}:subject:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(SubjectHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
    pub(super) fn credential_get(
        &self,
        actor_id: UserId,
        case: CaseId,
        id: ParticipantId,
        version: ParticipantRevision,
        at: OffsetDateTime,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, false)?;
        let result = credentials::read(&mut tx, case, id, version, self.hasher.as_ref())?;
        audit(
            &mut tx,
            &principal,
            "participant.credential_read",
            &format!(
                "case:{case}:participant:{id}:revision:{}:credential:{}",
                version.get(),
                result.reference.statement_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
