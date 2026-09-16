use super::*;
pub(super) fn stamp(
    tx: &mut impl GenericClient,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CaseDirectoryStamp, ApplicationError> {
    let mut stamp = directory_stamp_seed(hasher, case);
    for family in [0u8, 1] {
        let mut cursor: Option<Uuid> = None;
        loop {
            let sql = if family == 0 {
                "SELECT s.id,r.revision,r.values_digest,r.changed_at,r.changed_by,r.changed_by_email,
                    (r.values_digest=sha256(r.values_canonical) AND r.values_view=typed_subject_values(r.values_canonical)
                    AND r.subject_kind=r.values_view->>'kind'
                    AND r.display_name=CASE r.subject_kind WHEN 'natural_person' THEN COALESCE(r.values_view->'name'->>'known',r.values_view->'name'->>'label') ELSE r.values_view->>'name' END
                    AND r.name_known=(r.subject_kind='institutional_body' OR r.values_view->'name'?'known')
                    AND r.declared_identifier IS NOT DISTINCT FROM CASE r.subject_kind WHEN 'natural_person' THEN r.values_view->'curp'->>'known' ELSE r.values_view->'institutional_identifier'->>'known' END
                    AND r.identity_document_digest=decode(r.values_view->'identity_support'->>'digest','hex')
                    AND r.identity_document_locator=r.values_view->'identity_support'->>'locator') AS integrity
                 FROM case_subjects s JOIN LATERAL(SELECT * FROM case_subject_revisions WHERE subject_id=s.id ORDER BY revision DESC LIMIT 1) r ON TRUE
                 WHERE s.case_id=$1 AND ($2::uuid IS NULL OR s.id>$2) ORDER BY s.id LIMIT 64"
            } else {
                "SELECT p.id,r.* FROM case_participants p JOIN LATERAL(
                    SELECT revision,values_digest,changed_at,changed_by,changed_by_email,integrity FROM (
                        SELECT revision,values_digest,changed_at,changed_by,changed_by_email,
                            (participant_values_is_canonical(display_name,procedural_role,organization,legal_status,directory_status)
                                AND values_digest=sha256(participant_values_bytes(display_name,procedural_role,organization,legal_status,directory_status))) AS integrity
                            FROM case_participant_revisions WHERE participant_id=p.id
                        UNION ALL SELECT revision,values_digest,changed_at,changed_by,changed_by_email,
                            (values_digest=sha256(values_canonical) AND values_view=typed_participant_values(values_canonical)
                                AND subject_id=(values_view->'subject'->>'id')::uuid AND subject_revision=(values_view->'subject'->>'revision')::bigint
                                AND role_kind=values_view->'profile'->>'kind' AND organization IS NOT DISTINCT FROM values_view->>'organization'
                                AND directory_status=values_view->>'directory_status')
                            FROM case_participant_typed_revisions WHERE participant_id=p.id) h ORDER BY revision DESC LIMIT 1) r ON TRUE
                 WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2) ORDER BY p.id LIMIT 64"
            };
            let rows = tx
                .query(sql, &[&case.as_uuid(), &cursor])
                .map_err(|_| inconsistent())?;
            for row in &rows {
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
                let id: Uuid = row.get(0);
                stamp = advance_directory_stamp(
                    hasher,
                    stamp,
                    family,
                    id,
                    revision(row.get(1))?.get(),
                    digest(row.get(2))?,
                );
                cursor = Some(id);
            }
            if rows.len() < 64 {
                break;
            }
        }
    }
    Ok(stamp)
}
