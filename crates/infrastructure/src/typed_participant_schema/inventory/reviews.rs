use super::{digest, inconsistent, port};
use application::ApplicationError;
use postgres::GenericClient;
use serde_json::Value;
use uuid::Uuid;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one("SELECT
        EXISTS(SELECT 1 FROM subject_identity_reviews v LEFT JOIN case_subject_revisions r
            ON r.subject_id=v.subject_id AND r.revision=v.revision WHERE r.subject_id IS NULL)
        OR EXISTS(SELECT 1 FROM participant_identity_reviews v LEFT JOIN case_participant_typed_revisions r
            ON r.participant_id=v.participant_id AND r.revision=v.revision LEFT JOIN case_participants p ON p.id=r.participant_id
            WHERE r.participant_id IS NULL OR r.submission_revision<>v.revision OR v.submission_digest<>r.submission_digest
                OR octet_length(v.submission_canonical) NOT IN (167,615)
                OR substring(v.submission_canonical FROM 1 FOR 5)<>convert_to('PTXN1','UTF8')
                OR get_byte(v.submission_canonical,21) NOT IN (0,1)
                OR substring(v.submission_canonical FROM 6 FOR 16)<>uuid_send(p.case_id)
                OR substring(v.submission_canonical FROM 23 FOR 16)<>uuid_send(r.subject_id)
                OR typed_u32(v.submission_canonical,42)<>r.subject_revision
                OR typed_u32(v.submission_canonical,38)<>r.subject_revision-get_byte(v.submission_canonical,21)
                OR substring(v.submission_canonical FROM 47 FOR 32)<>substring(r.values_canonical FROM 26 FOR 32)
                OR substring(v.submission_canonical FROM 79 FOR 16)<>uuid_send(r.participant_id)
                OR typed_u32(v.submission_canonical,94)<>r.revision-1 OR typed_u32(v.submission_canonical,98)<>r.revision
                OR substring(v.submission_canonical FROM 103 FOR 32)<>r.values_digest
                OR substring(v.submission_canonical FROM 135 FOR 32)<>v.review_digest
                OR get_byte(v.submission_canonical,166) NOT IN (0,1)
                OR (get_byte(v.submission_canonical,166)=0 AND (octet_length(v.submission_canonical)<>167 OR r.credential_origin_revision IS NOT NULL))
                OR (get_byte(v.submission_canonical,166)=1 AND (octet_length(v.submission_canonical)<>615 OR r.credential_origin_revision IS DISTINCT FROM r.revision))
                OR (get_byte(v.submission_canonical,21)=1 AND NOT EXISTS(SELECT 1 FROM case_subject_revisions s
                    JOIN subject_identity_reviews sv ON sv.subject_id=s.subject_id AND sv.revision=s.revision
                    WHERE s.subject_id=r.subject_id AND s.revision=r.subject_revision AND sv.review_digest=v.review_digest
                        AND s.changed_at=r.changed_at AND s.changed_by=r.changed_by AND s.changed_by_email=r.changed_by_email)))",&[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    pages(client, true)?;
    pages(client, false)
}

fn pages<C: GenericClient>(client: &mut C, subject: bool) -> Result<(), ApplicationError> {
    let (table, id_column, join) = if subject {
        (
            "subject_identity_reviews",
            "subject_id",
            "JOIN case_subjects root ON root.id=v.subject_id",
        )
    } else {
        (
            "participant_identity_reviews",
            "participant_id",
            "JOIN case_participants root ON root.id=v.participant_id",
        )
    };
    let mut id = Uuid::nil();
    let mut revision = 0_i64;
    loop {
        let rows = client
            .query(
                &format!(
                    "SELECT v.{id_column},v.revision,v.review_canonical,v.review_digest,
            typed_review_values(v.review_canonical),root.case_id{} FROM {table} v {join}
            WHERE (v.{id_column},v.revision)>($1,$2) ORDER BY v.{id_column},v.revision LIMIT 64",
                    if subject {
                        ""
                    } else {
                        ",v.submission_canonical,v.submission_digest"
                    }
                ),
                &[&id, &revision],
            )
            .map_err(port)?;
        for row in &rows {
            let canonical: Vec<u8> = row.try_get(2).map_err(|_| inconsistent())?;
            let expected: Vec<u8> = row.try_get(3).map_err(|_| inconsistent())?;
            if !(46..=28941).contains(&canonical.len()) {
                return Err(inconsistent());
            }
            digest(&canonical, &expected)?;
            let view: Value = row.try_get(4).map_err(|_| inconsistent())?;
            let scope: Uuid = row.try_get(5).map_err(|_| inconsistent())?;
            candidates(client, scope, &view)?;
            if !subject {
                let submission: Vec<u8> = row.try_get(6).map_err(|_| inconsistent())?;
                let expected: Vec<u8> = row.try_get(7).map_err(|_| inconsistent())?;
                digest(&submission, &expected)?;
            }
            id = row.try_get(0).map_err(|_| inconsistent())?;
            revision = row.try_get(1).map_err(|_| inconsistent())?;
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}

fn candidates<C: GenericClient>(
    client: &mut C,
    scope: Uuid,
    view: &Value,
) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM jsonb_array_elements($2::jsonb->'different') c WHERE
        CASE (c->>'kind')::integer WHEN 0 THEN NOT EXISTS(SELECT 1 FROM case_subject_revisions r JOIN case_subjects s ON s.id=r.subject_id
            WHERE s.case_id=$1 AND s.id=(c->>'id')::uuid AND r.revision=(c->>'revision')::bigint)
        WHEN 1 THEN NOT EXISTS(SELECT 1 FROM case_participant_revisions r JOIN case_participants p ON p.id=r.participant_id
            WHERE p.case_id=$1 AND p.id=(c->>'id')::uuid AND r.revision=(c->>'revision')::bigint) ELSE TRUE END
        OR NOT EXISTS(SELECT 1 FROM documents d WHERE d.case_id=$1 AND d.id=(c->'support'->>'document_id')::uuid
            AND d.version=(c->'support'->>'version')::bigint AND d.digest=decode(c->'support'->>'digest','hex')))",
        &[&scope,&view]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    Ok(())
}
