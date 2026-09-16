SELECT p.case_id,p.id AS participant_id,h.*,
    CASE WHEN h.typed THEN s.display_name ELSE h.display_name END AS overview_name,
    s.values_digest AS subject_values_digest,bs.case_id AS subject_case_id,
    s.changed_at AS subject_changed_at,
    CASE WHEN h.typed THEN
        s.values_digest=pg_catalog.sha256(s.values_canonical)
        AND s.values_digest=h.expected_subject_digest
        AND s.display_name=CASE s.subject_kind
            WHEN 'natural_person' THEN COALESCE(s.values_view->'name'->>'known',s.values_view->'name'->>'label')
            ELSE s.values_view->>'name' END
        AND participant_text_valid(s.changed_by_email,2147483647)
    ELSE TRUE END AS subject_integrity_ok
FROM case_participants p JOIN LATERAL (
    SELECT revision,FALSE AS typed,display_name,procedural_role,organization,directory_status,
        NULL::uuid AS subject_id,NULL::bigint AS subject_revision,NULL::bytea AS expected_subject_digest,
        changed_at,
        CASE WHEN participant_values_is_canonical(display_name,procedural_role,organization,legal_status,directory_status)
            THEN values_digest=pg_catalog.sha256(participant_values_bytes(display_name,procedural_role,organization,legal_status,directory_status))
                AND participant_text_valid(changed_by_email,2147483647)
            ELSE FALSE END AS integrity_ok
    FROM case_participant_revisions WHERE participant_id=p.id
    UNION ALL
    SELECT revision,TRUE,NULL::text,role_kind,organization,directory_status,
        subject_id,subject_revision,decode(values_view->'subject'->>'digest','hex'),changed_at,
        values_digest=pg_catalog.sha256(values_canonical)
            AND subject_id=(values_view->'subject'->>'id')::uuid
            AND subject_revision=(values_view->'subject'->>'revision')::bigint
            AND role_kind=values_view->'profile'->>'kind'
            AND organization IS NOT DISTINCT FROM values_view->>'organization'
            AND directory_status=values_view->>'directory_status'
            AND participant_text_valid(changed_by_email,2147483647)
    FROM case_participant_typed_revisions WHERE participant_id=p.id
    ORDER BY revision DESC LIMIT 1
) h ON TRUE
LEFT JOIN case_subject_revisions s ON s.subject_id=h.subject_id AND s.revision=h.subject_revision
LEFT JOIN case_subjects bs ON bs.id=s.subject_id
WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2)
    AND ($3::text IS NULL OR strpos((CASE WHEN h.typed THEN s.display_name ELSE h.display_name END) COLLATE "C",$3)>0)
    AND ($4::text IS NULL OR (NOT h.typed AND h.procedural_role COLLATE "C"=$4))
    AND ($5::text IS NULL OR h.directory_status COLLATE "C"=$5)
    AND ($6::text IS NULL OR (h.typed AND h.procedural_role=$6))
    AND ($7::smallint=0 OR ($7=1 AND NOT h.typed) OR ($7=2 AND h.typed))
ORDER BY p.id LIMIT $8
