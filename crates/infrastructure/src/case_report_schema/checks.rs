use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<String> {
    let checks: &[&str] = match table {
        "case_report_jobs" => &[
            "((jsonb_typeof(principal) = 'object'::text) AND (octet_length((principal)::text) <= 2048))",
            "(account_revision >= 0)",
            "(auth_generation >= 0)",
            "(scope = ANY (ARRAY['office'::text, 'assigned_cases'::text]))",
            "((jsonb_typeof(command) = 'object'::text) AND (octet_length((command)::text) <= 4096))",
            "(octet_length(request_digest) = 32)",
            "(state = ANY (ARRAY['queued'::text, 'capturing'::text, 'rendering'::text, 'retry_capturing'::text, 'retry_rendering'::text, 'ready'::text, 'failed'::text, 'revoked'::text]))",
            "(failure = ANY (ARRAY['temporary_unavailable'::text, 'render_unavailable'::text, 'render_failed'::text, 'capacity_exceeded'::text, 'invalid_stored_capture'::text, 'access_revoked'::text]))",
            "(lease_generation >= 0)",
            "((attempts >= 0) AND (attempts <= 5))",
            "((cardinality(case_ids) <= 1000) AND (array_position(case_ids, NULL::uuid) IS NULL))",
            "(((lease_attempt IS NULL) = (lease_token IS NULL)) AND ((lease_token IS NULL) = (lease_expires_at IS NULL)))",
            "((state = ANY (ARRAY['capturing'::text, 'rendering'::text])) = ((lease_attempt IS NOT NULL) AND (lease_token IS NOT NULL) AND (lease_expires_at IS NOT NULL)))",
            "((state = ANY (ARRAY['retry_capturing'::text, 'retry_rendering'::text])) = (retry_at IS NOT NULL))",
            "((state = ANY (ARRAY['failed'::text, 'revoked'::text, 'retry_capturing'::text, 'retry_rendering'::text])) = (failure IS NOT NULL))",
        ],
        "case_report_snapshots" => &[
            "(octet_length(digest) = 32)", "(octet_length(plaintext_digest) = 32)",
            "(octet_length(wrapped_dek) = 60)",
            "((octet_length(payload) >= 28) AND (octet_length(payload) <= 8388636))",
        ],
        "case_report_artifacts" => &[
            "(format = ANY (ARRAY['pdf'::text, 'csv'::text]))",
            "(octet_length(snapshot_digest) = 32)", "(octet_length(digest) = 32)",
            "((bytes >= 1) AND (bytes <= 16777216))",
            "(octet_length(wrapped_dek) = 60)", "(octet_length(payload) = (bytes + 28))",
        ],
        "case_report_notices" => &["(kind = ANY (ARRAY['ready'::text, 'failed'::text]))"],
        _ => &[],
    };
    checks.iter().map(|check| (*check).to_owned()).collect()
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
