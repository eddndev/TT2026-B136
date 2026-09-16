use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, key, checks) in [
        (TABLES[0], vec!["id"], 1_i64),
        (TABLES[1], vec!["subject_id", "revision"], 4),
        (TABLES[2], vec!["participant_id", "revision"], 8),
        (TABLES[3], vec!["subject_id", "revision"], 2),
        (TABLES[4], vec!["participant_id", "revision"], 4),
        (TABLES[5], vec!["participant_id", "revision"], 7),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
            WHERE c.conrelid=$1::text::regclass AND c.contype='p' AND c.convalidated
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$2::text[])
            AND (SELECT count(*) FROM pg_catalog.pg_constraint WHERE conrelid=$1::text::regclass
                AND contype='c' AND convalidated)=$3",&[&table,&key,&checks]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, names) in [
        (
            "case_subjects",
            vec!["case_subjects_initial_revision_check"],
        ),
        (
            "case_subject_revisions",
            vec![
                "case_subject_revisions_check",
                "case_subject_revisions_revision_check",
                "case_subject_revisions_values_canonical_check",
                "subject_projection",
            ],
        ),
        (
            "case_participant_typed_revisions",
            vec![
                "case_participant_typed_revisions_check",
                "case_participant_typed_revisions_check1",
                "case_participant_typed_revisions_check2",
                "case_participant_typed_revisions_revision_check",
                "case_participant_typed_revisions_submission_digest_check",
                "case_participant_typed_revisions_values_canonical_check",
                "typed_initial_active",
                "typed_projection",
            ],
        ),
        (
            "subject_identity_reviews",
            vec![
                "subject_identity_reviews_check",
                "subject_identity_reviews_review_canonical_check",
            ],
        ),
        (
            "participant_identity_reviews",
            vec![
                "participant_identity_reviews_check",
                "participant_identity_reviews_check1",
                "participant_identity_reviews_review_canonical_check",
                "participant_identity_reviews_submission_canonical_check",
            ],
        ),
        (
            "participant_credential_evidence",
            vec![
                "participant_credential_evidence_accepted_at_nanoseconds_check",
                "participant_credential_evidence_certificate_der_check",
                "participant_credential_evidence_check",
                "participant_credential_evidence_check1",
                "participant_credential_evidence_declaration_check",
                "participant_credential_evidence_signature_check",
                "participant_credential_window",
            ],
        ),
    ] {
        let valid:bool=client.query_one("SELECT count(*)=cardinality($2::text[]) FROM pg_catalog.pg_constraint
            WHERE conrelid=$1::text::regclass AND contype='c' AND convalidated AND conname::text=ANY($2::text[])",
            &[&table,&names]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, target, columns, targets, deferred) in [
        (TABLES[0], "cases", vec!["case_id"], vec!["id"], false),
        (
            TABLES[0],
            TABLES[1],
            vec!["id", "initial_revision"],
            vec!["subject_id", "revision"],
            true,
        ),
        (TABLES[1], TABLES[0], vec!["subject_id"], vec!["id"], false),
        (TABLES[1], "users", vec!["changed_by"], vec!["id"], false),
        (
            TABLES[1],
            "documents",
            vec!["identity_document_id", "identity_document_version"],
            vec!["id", "version"],
            false,
        ),
        (
            TABLES[1],
            TABLES[3],
            vec!["subject_id", "revision"],
            vec!["subject_id", "revision"],
            true,
        ),
        (
            TABLES[2],
            "case_participants",
            vec!["participant_id"],
            vec!["id"],
            false,
        ),
        (TABLES[2], "users", vec!["changed_by"], vec!["id"], false),
        (
            TABLES[2],
            TABLES[1],
            vec!["subject_id", "subject_revision"],
            vec!["subject_id", "revision"],
            false,
        ),
        (
            TABLES[2],
            TABLES[4],
            vec!["participant_id", "submission_revision"],
            vec!["participant_id", "revision"],
            true,
        ),
        (
            TABLES[2],
            TABLES[5],
            vec!["participant_id", "credential_origin_revision"],
            vec!["participant_id", "revision"],
            true,
        ),
        (
            TABLES[3],
            TABLES[1],
            vec!["subject_id", "revision"],
            vec!["subject_id", "revision"],
            true,
        ),
        (
            TABLES[4],
            TABLES[2],
            vec!["participant_id", "revision"],
            vec!["participant_id", "revision"],
            true,
        ),
        (
            TABLES[5],
            TABLES[2],
            vec!["participant_id", "revision"],
            vec!["participant_id", "revision"],
            true,
        ),
        (
            TABLES[5],
            "participant_credential_trust_revisions",
            vec!["deployment_id", "trust_revision"],
            vec!["deployment_id", "revision"],
            false,
        ),
    ] {
        foreign_key(client, table, target, &columns, &targets, deferred)?;
    }
    Ok(())
}

fn foreign_key<C: GenericClient>(
    client: &mut C,
    table: &str,
    target: &str,
    columns: &[&str],
    targets: &[&str],
    deferred: bool,
) -> Result<(), ApplicationError> {
    let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
        WHERE c.conrelid=$1::text::regclass AND c.confrelid=$2::text::regclass AND c.contype='f' AND c.convalidated
        AND c.condeferrable=$5 AND c.condeferred=$5 AND c.confdeltype='a' AND c.confupdtype='a'
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[]
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$4::text[])",
        &[&table,&target,&columns,&targets,&deferred]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
