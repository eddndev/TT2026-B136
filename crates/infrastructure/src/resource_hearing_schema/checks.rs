use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<String> {
    if table == TABLES[0] {
        return vec!["(initial_revision = 1)".into()];
    }
    [
        "(revision = 1)",
        "((jsonb_typeof(values_view) = 'object'::text) AND (octet_length((values_view)::text) <= 65536))",
        "((octet_length(values_canonical) >= 6) AND (octet_length(values_canonical) <= 65536))",
        "((octet_length(submission_canonical) >= 5) AND (octet_length(submission_canonical) <= 1048576))",
        "(submission_digest = sha256(submission_canonical))",
        "((octet_length(capture_canonical) >= 5) AND (octet_length(capture_canonical) <= 1048576))",
        "(capture_digest = sha256(capture_canonical))",
        "(octet_length(recorded_administration_title) <= 800)",
        "(octet_length(recorded_administration_reference) <= 400)",
        "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))",
        "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))",
        "((octet_length(recorded_by_email) >= 1) AND (octet_length(recorded_by_email) <= 1280))",
        "(((recorded_administration_revision IS NULL) AND (recorded_administration_digest IS NULL) AND (recorded_administration_title IS NOT NULL) AND (recorded_administration_reference IS NOT NULL)) OR ((recorded_administration_revision >= 1) AND (recorded_administration_revision <= '4294967295'::bigint) AND (recorded_administration_revision IS NOT NULL) AND (recorded_administration_digest IS NOT NULL) AND (octet_length(recorded_administration_digest) = 32) AND (recorded_administration_title IS NULL) AND (recorded_administration_reference IS NULL)))",
        "(num_nonnulls(act_id, act_revision, act_resource_revision, act_capture_digest) = ANY (ARRAY[0, 4]))",
        "((resource_revision >= 1) AND (resource_revision <= '4294967295'::bigint))",
        "((act_revision >= 1) AND (act_revision <= '4294967295'::bigint))",
        "((act_resource_revision >= 2) AND (act_resource_revision <= '4294967295'::bigint))",
        "((recorded_resource_revision >= 1) AND (recorded_resource_revision <= '4294967295'::bigint))",
        "(octet_length(resource_capture_digest) = 32)",
        "(octet_length(act_capture_digest) = 32)",
        "(octet_length(recorded_resource_capture_digest) = 32)",
    ].into_iter().map(str::to_owned).collect()
}
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client
            .query(
                "SELECT pg_get_expr(c.conbin,c.conrelid),c.convalidated AND NOT c.connoinherit
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.contype='c'",
                &[&table],
            )
            .map_err(port)?;
        let mut actual: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
        let mut expected = expected(table);
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
