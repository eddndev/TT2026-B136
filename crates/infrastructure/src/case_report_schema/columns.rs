use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [(&'static str, &'static str, bool)] {
    match table {
        "case_report_jobs" => &[
            ("id", "uuid", true),
            ("requester_id", "uuid", true),
            ("principal", "jsonb", true),
            ("account_revision", "bigint", true),
            ("auth_generation", "bigint", true),
            ("scope", "text", true),
            ("operation_id", "uuid", true),
            ("command", "jsonb", true),
            ("request_digest", "bytea", true),
            ("requested_at", "text", true),
            ("updated_at", "text", true),
            ("state", "text", true),
            ("failure", "text", false),
            ("retry_at", "text", false),
            ("lease_attempt", "uuid", false),
            ("lease_token", "uuid", false),
            ("lease_generation", "bigint", true),
            ("lease_expires_at", "text", false),
            ("attempts", "integer", true),
            ("case_ids", "uuid[]", true),
        ],
        "case_report_snapshots" => &[
            ("report_id", "uuid", true),
            ("digest", "bytea", true),
            ("plaintext_digest", "bytea", true),
            ("checked_at", "text", true),
            ("wrapped_dek", "bytea", true),
            ("payload", "bytea", true),
        ],
        "case_report_artifacts" => &[
            ("report_id", "uuid", true),
            ("format", "text", true),
            ("snapshot_digest", "bytea", true),
            ("digest", "bytea", true),
            ("bytes", "bigint", true),
            ("wrapped_dek", "bytea", true),
            ("payload", "bytea", true),
        ],
        "case_report_notices" => &[
            ("report_id", "uuid", true),
            ("kind", "text", true),
            ("created_at", "text", true),
            ("read_at", "text", false),
        ],
        _ => &[],
    }
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let expected = expected(table);
        let rows = client.query(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation ELSE a.attcollation=0 END,
                (SELECT pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum", &[&table],
        ).map_err(port)?;
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
                        || row.get::<_, Option<String>>(6).as_deref()
                            != match *name {
                                "lease_generation" | "attempts" => Some("0"),
                                "case_ids" => Some("'{}'::uuid[]"),
                                _ => None,
                            }
                })
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
