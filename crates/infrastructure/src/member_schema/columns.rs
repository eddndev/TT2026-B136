use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [(&'static str, &'static str, bool)] {
    match table {
        "users" => &[
            ("id", "uuid", true),
            ("email", "text", true),
            ("password_hash", "text", true),
            ("role", "text", true),
            ("active", "boolean", true),
            ("protected_totp_secret", "bytea", true),
            ("recovery_codes", "jsonb", true),
            ("revision", "bigint", true),
            ("created_at", "timestamp with time zone", true),
            ("updated_at", "timestamp with time zone", true),
            ("auth_generation", "bigint", true),
        ],
        "case_memberships" => &[
            ("case_id", "uuid", true),
            ("user_id", "uuid", true),
            ("assigned_at", "timestamp with time zone", true),
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
                                "active" => Some("true"),
                                "revision" | "auth_generation" => Some("0"),
                                "created_at" | "updated_at" | "assigned_at" => {
                                    Some("CURRENT_TIMESTAMP")
                                }
                                _ => None,
                            }
                })
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
