use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(_table: &str) -> Vec<String> {
    [
        "((document_version >= 1) AND (document_version <= '4294967295'::bigint))",
        "(failure = ANY (ARRAY['malformed_vault'::text, 'authentication_failed'::text, 'digest_mismatch'::text, 'snapshot_changed'::text]))",
        "(ROW(recorded_at_seconds, recorded_at_nanoseconds) >= ROW(detected_at_seconds, detected_at_nanoseconds))",
        "(octet_length(expected_digest) = 32)",
        "(octet_length(observed_snapshot_digest) = 32)",
        "(octet_length(capture_canonical) = 178)",
        "(capture_digest = sha256(capture_canonical))",
        "((detected_at_seconds >= '-62135596800'::bigint) AND (detected_at_seconds <= '253402300799'::bigint))",
        "((detected_at_nanoseconds >= 0) AND (detected_at_nanoseconds <= 999999999))",
        "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))",
        "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))",
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
