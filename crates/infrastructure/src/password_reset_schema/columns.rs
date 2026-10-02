use super::{incomplete, port, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) const COLUMNS: &[(&str, &str, bool)] = &[
    ("id", "uuid", true),
    ("digest", "bytea", true),
    ("user_id", "uuid", true),
    ("email", "text", true),
    ("auth_generation", "bigint", true),
    ("issued_at", "timestamp with time zone", true),
    ("expires_at", "timestamp with time zone", true),
    ("cancelled_at", "timestamp with time zone", false),
    ("consumed_at", "timestamp with time zone", false),
    ("consumed_revision", "bigint", false),
    ("consumed_generation", "bigint", false),
    ("audit_sequence", "bigint", false),
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client.query(
        "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
        a.attgenerated::text,a.attidentity::text,
        CASE WHEN a.atttypid='text'::regtype THEN a.attcollation='pg_catalog.\"default\"'::regcollation
            ELSE a.attcollation=0 END,
        EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
        FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
            AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum", &[&TABLE],
    ).map_err(port)?;
    if rows.len() != COLUMNS.len()
        || rows
            .iter()
            .zip(COLUMNS)
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
    Ok(())
}
