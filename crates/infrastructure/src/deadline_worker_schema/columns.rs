use super::{incomplete, port, ATTEMPTS, RESULTS};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) const RESULT_COLUMNS: &[(&str, &str, bool)] = &[
    ("job_id", "uuid", true),
    ("base_revision", "bigint", true),
    ("base_submission_digest", "bytea", true),
    ("base_capture_digest", "bytea", true),
    ("outcome", "text", true),
    ("result_revision", "bigint", false),
    ("result_submission_digest", "bytea", false),
    ("result_capture_digest", "bytea", false),
    ("checked_observations_canonical", "bytea", false),
    ("checked_administration_revision", "bigint", false),
    ("checked_administration_evidence_digest", "bytea", false),
    ("completed_at_seconds", "bigint", true),
    ("completed_at_nanoseconds", "integer", true),
];
pub(super) const ATTEMPT_COLUMNS: &[(&str, &str, bool)] = &[
    ("attempt_id", "uuid", true),
    ("job_id", "uuid", true),
    ("attempt_number", "bigint", true),
    ("checked_base_revision", "bigint", false),
    ("checked_base_submission_digest", "bytea", false),
    ("checked_base_capture_digest", "bytea", false),
    ("failure_kind", "text", true),
    ("error_code", "text", true),
    ("failed_at_seconds", "bigint", true),
    ("failed_at_nanoseconds", "integer", true),
    ("retry_at_seconds", "bigint", true),
    ("retry_at_nanoseconds", "integer", true),
];

pub(super) fn names(table: &str) -> Vec<&'static str> {
    let columns = if table == RESULTS {
        RESULT_COLUMNS
    } else {
        ATTEMPT_COLUMNS
    };
    columns.iter().map(|column| column.0).collect()
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, expected) in [(RESULTS, RESULT_COLUMNS), (ATTEMPTS, ATTEMPT_COLUMNS)] {
        let rows = client
            .query(
                "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation
                    ELSE a.attcollation=0 END,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
                &[&table],
            )
            .map_err(port)?;
        if rows.len() != expected.len()
            || rows
                .iter()
                .zip(expected)
                .any(|(row, (name, kind, required))| {
                    row.get::<_, String>(0) != *name
                        || row.get::<_, String>(1) != *kind
                        || row.get::<_, bool>(2) != *required
                        || !row.get::<_, String>(3).is_empty()
                        || !row.get::<_, String>(4).is_empty()
                        || !row.get::<_, bool>(5)
                        || row.get::<_, bool>(6)
                })
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
