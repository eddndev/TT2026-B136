//! Exact deparsed CHECK expressions for both immutable worker ledgers.
use super::{incomplete, port, ATTEMPTS, RESULTS};
use application::ApplicationError;
use postgres::GenericClient;

const CHECKS: &[(&str, &str, &str)] = &[
    (
        ATTEMPTS,
        "deadline_worker_attempt_base",
        r#"(((num_nonnulls(checked_base_revision, checked_base_submission_digest, checked_base_capture_digest) = 0) OR ((checked_base_revision >= 1) AND (checked_base_revision <= '4294967295'::bigint) AND (octet_length(checked_base_submission_digest) = 32) AND (octet_length(checked_base_capture_digest) = 32))) IS TRUE)"#,
    ),
    (
        ATTEMPTS,
        "deadline_worker_attempt_failure",
        r#"(((((failure_kind COLLATE "C") = 'inconsistent'::text) AND ((error_code COLLATE "C") = ANY (ARRAY['invalid_stored_evidence'::text, 'invalid_durable_job'::text]))) OR (((failure_kind COLLATE "C") = 'transient'::text) AND ((error_code COLLATE "C") = ANY (ARRAY['lock_unavailable'::text, 'database_unavailable'::text, 'transaction_interrupted'::text, 'execution_failed'::text])))) IS TRUE)"#,
    ),
    (
        ATTEMPTS,
        "deadline_worker_attempt_nanoseconds",
        r#"((failed_at_nanoseconds >= 0) AND (failed_at_nanoseconds <= 999999999) AND (retry_at_nanoseconds >= 0) AND (retry_at_nanoseconds <= 999999999))"#,
    ),
    (
        ATTEMPTS,
        "deadline_worker_attempt_positive",
        r#"(attempt_number > 0)"#,
    ),
    (
        ATTEMPTS,
        "deadline_worker_attempt_retry",
        r#"(ROW(retry_at_seconds, retry_at_nanoseconds) > ROW(failed_at_seconds, failed_at_nanoseconds))"#,
    ),
    (
        ATTEMPTS,
        "deadline_worker_attempt_seconds",
        r#"((failed_at_seconds >= '-62135596800'::bigint) AND (failed_at_seconds <= '253402300799'::bigint) AND (retry_at_seconds >= '-62135596800'::bigint) AND (retry_at_seconds <= '253402300799'::bigint))"#,
    ),
    (
        RESULTS,
        "deadline_worker_result_base",
        r#"(((base_revision >= 1) AND (base_revision <= '4294967295'::bigint) AND (octet_length(base_submission_digest) = 32) AND (octet_length(base_capture_digest) = 32)) IS TRUE)"#,
    ),
    (
        RESULTS,
        "deadline_worker_result_nanoseconds",
        r#"((completed_at_nanoseconds >= 0) AND (completed_at_nanoseconds <= 999999999))"#,
    ),
    (
        RESULTS,
        "deadline_worker_result_outcome",
        r#"(((outcome COLLATE "C") = ANY (ARRAY['revision'::text, 'retired'::text, 'already_observed'::text, 'dependency_not_selected'::text, 'already_initialized'::text])) IS TRUE)"#,
    ),
    (
        RESULTS,
        "deadline_worker_result_seconds",
        r#"((completed_at_seconds >= '-62135596800'::bigint) AND (completed_at_seconds <= '253402300799'::bigint))"#,
    ),
    (
        RESULTS,
        "deadline_worker_result_shape",
        r#"((((outcome = 'revision'::text) AND (result_revision = (base_revision + 1)) AND (result_revision <= '4294967295'::bigint) AND (octet_length(result_submission_digest) = 32) AND (octet_length(result_capture_digest) = 32) AND (num_nonnulls(checked_observations_canonical, checked_administration_revision, checked_administration_evidence_digest) = 0)) OR ((outcome = ANY (ARRAY['retired'::text, 'already_initialized'::text])) AND (num_nonnulls(result_revision, result_submission_digest, result_capture_digest, checked_observations_canonical, checked_administration_revision, checked_administration_evidence_digest) = 0)) OR ((outcome = ANY (ARRAY['already_observed'::text, 'dependency_not_selected'::text])) AND (num_nonnulls(result_revision, result_submission_digest, result_capture_digest) = 0) AND (octet_length(checked_observations_canonical) >= 111) AND (octet_length(checked_observations_canonical) <= 446) AND (SUBSTRING(checked_observations_canonical FROM 1 FOR 5) = convert_to('DLOB1'::text, 'UTF8'::name)) AND ((checked_administration_revision IS NULL) OR ((checked_administration_revision >= 1) AND (checked_administration_revision <= '4294967295'::bigint))) AND (octet_length(checked_administration_evidence_digest) = 32))) IS TRUE)"#,
    ),
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT t.relname::text,c.conname::text,pg_catalog.pg_get_expr(c.conbin,c.conrelid),
            c.convalidated AND NOT c.connoinherit AND c.conislocal
                AND c.coninhcount=0 AND c.conparentid=0 AND c.connamespace=t.relnamespace
                AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
        FROM pg_catalog.pg_constraint c JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
        WHERE c.conrelid IN ($1::text::regclass,$2::text::regclass) AND c.contype='c'",
            &[&RESULTS, &ATTEMPTS],
        )
        .map_err(port)?;
    if rows.len() != CHECKS.len()
        || CHECKS.iter().any(|(table, name, expected)| {
            !rows.iter().any(|row| {
                row.get::<_, String>(0) == *table
                    && row.get::<_, String>(1) == *name
                    && row.get::<_, String>(2) == *expected
                    && row.get::<_, bool>(3)
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}
