use super::{inconsistent, port};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one("SELECT
        EXISTS(SELECT 1 FROM case_subjects s LEFT JOIN cases c ON c.id=s.case_id
            LEFT JOIN (SELECT subject_id,min(revision) first,max(revision) last,count(*) total,count(DISTINCT subject_kind) kinds
                FROM case_subject_revisions GROUP BY subject_id) r ON r.subject_id=s.id
            WHERE c.id IS NULL OR s.initial_revision<>1 OR r.first IS DISTINCT FROM 1 OR r.last<>r.total OR r.kinds<>1
                OR NOT EXISTS(SELECT 1 FROM case_participant_typed_revisions t WHERE t.subject_id=s.id))
        OR EXISTS(SELECT 1 FROM case_subject_revisions r LEFT JOIN case_subjects s ON s.id=r.subject_id
            LEFT JOIN users u ON u.id=r.changed_by LEFT JOIN documents d ON d.id=r.identity_document_id AND d.version=r.identity_document_version
            LEFT JOIN subject_identity_reviews v ON v.subject_id=r.subject_id AND v.revision=r.revision
            WHERE s.id IS NULL OR u.id IS NULL OR v.subject_id IS NULL OR d.id IS NULL OR d.case_id<>s.case_id
                OR d.digest<>r.identity_document_digest OR r.revision NOT BETWEEN 1 AND 4294967295
                OR r.values_view IS DISTINCT FROM typed_subject_values(r.values_canonical)
                OR r.subject_kind IS DISTINCT FROM r.values_view->>'kind'
                OR r.display_name IS DISTINCT FROM CASE r.subject_kind WHEN 'natural_person' THEN
                    COALESCE(r.values_view->'name'->>'known',r.values_view->'name'->>'label') ELSE r.values_view->>'name' END
                OR r.name_known IS DISTINCT FROM (r.subject_kind='institutional_body' OR r.values_view->'name'?'known')
                OR r.declared_identifier IS DISTINCT FROM CASE r.subject_kind WHEN 'natural_person' THEN r.values_view->'curp'->>'known'
                    ELSE r.values_view->'institutional_identifier'->>'known' END
                OR r.identity_document_id IS DISTINCT FROM (r.values_view->'identity_support'->>'document_id')::uuid
                OR r.identity_document_version IS DISTINCT FROM (r.values_view->'identity_support'->>'version')::bigint
                OR r.identity_document_digest IS DISTINCT FROM decode(r.values_view->'identity_support'->>'digest','hex')
                OR r.identity_document_locator IS DISTINCT FROM r.values_view->'identity_support'->>'locator')
        OR EXISTS(SELECT 1 FROM case_participant_typed_revisions r
            LEFT JOIN case_participants p ON p.id=r.participant_id LEFT JOIN users u ON u.id=r.changed_by
            LEFT JOIN case_subjects s ON s.id=r.subject_id LEFT JOIN case_subject_revisions b ON b.subject_id=r.subject_id AND b.revision=r.subject_revision
            LEFT JOIN participant_identity_reviews v ON v.participant_id=r.participant_id AND v.revision=r.submission_revision
            LEFT JOIN participant_credential_evidence e ON e.participant_id=r.participant_id AND e.revision=r.credential_origin_revision
            WHERE p.id IS NULL OR u.id IS NULL OR b.subject_id IS NULL OR s.case_id<>p.case_id OR v.participant_id IS NULL
                OR (r.credential_origin_revision IS NOT NULL AND e.participant_id IS NULL)
                OR r.values_view IS DISTINCT FROM typed_participant_values(r.values_canonical)
                OR r.subject_id IS DISTINCT FROM (r.values_view->'subject'->>'id')::uuid
                OR r.subject_revision IS DISTINCT FROM (r.values_view->'subject'->>'revision')::bigint
                OR b.values_digest IS DISTINCT FROM decode(r.values_view->'subject'->>'digest','hex')
                OR r.role_kind IS DISTINCT FROM r.values_view->'profile'->>'kind'
                OR r.organization IS DISTINCT FROM r.values_view->>'organization'
                OR r.directory_status IS DISTINCT FROM r.values_view->>'directory_status'
                OR r.revision NOT BETWEEN 1 AND 4294967295 OR (r.revision=1 AND r.directory_status<>'active')
                OR r.submission_revision NOT BETWEEN 1 AND r.revision OR r.submission_digest<>v.submission_digest
                OR (r.credential_origin_revision IS NOT NULL AND r.credential_origin_revision NOT BETWEEN 1 AND r.revision)
                OR (r.role_kind IN ('defense_counsel','control_judge') AND r.credential_origin_revision IS NULL)
                OR (r.role_kind='trial_court' AND b.subject_kind<>'institutional_body')
                OR (r.role_kind NOT IN ('trial_court','other') AND b.subject_kind<>'natural_person')
                OR (b.subject_kind='institutional_body' AND r.credential_origin_revision IS NOT NULL))
        OR EXISTS(SELECT 1 FROM case_participant_typed_revisions GROUP BY participant_id HAVING count(DISTINCT subject_id)<>1)
        OR EXISTS(SELECT 1 FROM case_participant_typed_revisions r WHERE r.revision=(SELECT max(h.revision)
            FROM case_participant_typed_revisions h WHERE h.participant_id=r.participant_id)
            GROUP BY subject_id,role_kind HAVING count(*)>1)
        OR EXISTS(SELECT 1 FROM case_participant_typed_revisions r LEFT JOIN case_participant_typed_revisions prev
            ON prev.participant_id=r.participant_id AND prev.revision=r.revision-1
            LEFT JOIN case_participant_revisions m ON m.participant_id=r.participant_id AND m.revision=r.revision-1
            WHERE (r.submission_revision=r.revision AND (
                (r.credential_origin_revision IS NOT NULL AND r.credential_origin_revision<>r.revision)
                OR (r.revision>1 AND r.directory_status IS DISTINCT FROM COALESCE(prev.directory_status,m.directory_status))))
            OR (r.submission_revision<>r.revision AND (prev.participant_id IS NULL
                OR r.submission_revision<>prev.submission_revision OR r.submission_digest<>prev.submission_digest
                OR r.credential_origin_revision IS DISTINCT FROM prev.credential_origin_revision
                OR set_byte(r.values_canonical,57,get_byte(prev.values_canonical,57))<>prev.values_canonical)))
        OR EXISTS(SELECT 1 FROM case_participant_typed_revisions r JOIN case_participants p ON p.id=r.participant_id
            CROSS JOIN LATERAL(SELECT r.values_view->'role_support' support
                UNION ALL SELECT r.values_view->'profile'->'contact'->'documented' WHERE r.values_view->'profile'->'contact'?'documented'
                UNION ALL SELECT r.values_view->'profile'->'protection'->'documented' WHERE r.values_view->'profile'->'protection'?'documented') refs
            WHERE NOT EXISTS(SELECT 1 FROM documents d WHERE d.case_id=p.case_id AND d.id=(refs.support->>'document_id')::uuid
                AND d.version=(refs.support->>'version')::bigint AND d.digest=decode(refs.support->>'digest','hex')))",&[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    Ok(())
}
