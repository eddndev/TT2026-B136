use application::ApplicationError;
use postgres::GenericClient;

use super::{constraints, invalid, port};

type Column = (&'static str, &'static str, bool, Option<&'static str>);

const USERS: &[Column] = &[
    ("id", "uuid", true, None),
    ("email", "text", true, None),
    ("password_hash", "text", true, None),
    ("role", "text", true, None),
    ("active", "boolean", true, Some("true")),
    ("protected_totp_secret", "bytea", true, None),
    ("recovery_codes", "jsonb", true, None),
    ("revision", "bigint", true, Some("0")),
    (
        "created_at",
        "timestamp with time zone",
        true,
        Some("CURRENT_TIMESTAMP"),
    ),
    (
        "updated_at",
        "timestamp with time zone",
        true,
        Some("CURRENT_TIMESTAMP"),
    ),
    ("auth_generation", "bigint", true, Some("0")),
];
const AUDIT: &[Column] = &[
    ("sequence", "bigint", true, None),
    ("timestamp", "text", true, None),
    ("actor", "text", true, None),
    ("action", "text", true, None),
    ("resource", "text", true, None),
    ("chain", "bytea", true, None),
    ("context_json", "text", false, None),
];

pub(super) fn validate(
    client: &mut impl GenericClient,
    database: &str,
    role: &str,
) -> Result<Vec<String>, ApplicationError> {
    target(client, database, role)?;
    relations(client)
}

pub(super) fn target(
    client: &mut impl GenericClient,
    database: &str,
    role: &str,
) -> Result<(), ApplicationError> {
    let row = client
        .query_one(
            "SELECT pg_catalog.current_database(), current_user::text, session_user::text,
         pg_catalog.current_schema(), pg_catalog.current_setting('search_path')",
            &[],
        )
        .map_err(port)?;
    let path: String = row.get(4);
    if row.get::<_, String>(0) != database
        || row.get::<_, String>(1) != role
        || row.get::<_, String>(2) != role
        || row.get::<_, Option<String>>(3).as_deref() != Some("qadra_ca")
        || !path.split(',').map(str::trim).eq(["qadra_ca", "pg_temp"])
    {
        return Err(invalid(
            "CA identity requires its expected database, role and fixed search path",
        ));
    }
    Ok(())
}

fn relations(client: &mut impl GenericClient) -> Result<Vec<String>, ApplicationError> {
    let schema = client
        .query_opt(
            "SELECT n.oid::text, n.nspowner::text FROM pg_catalog.pg_namespace n
         JOIN pg_catalog.pg_database d ON d.datname=pg_catalog.current_database()
         WHERE n.nspname='qadra_ca' AND n.nspowner=d.datdba",
            &[],
        )
        .map_err(port)?
        .ok_or_else(|| invalid("CA identity schema must belong to the database owner"))?;
    let owner: String = schema.get(1);
    let mut identity = vec![schema.get(0), owner.clone()];
    for (table, expected) in [("users", USERS), ("audit_events", AUDIT)] {
        let record = client
            .query_opt(
                "SELECT c.oid::text, c.relowner::text,
             c.relkind='r' AND c.relpersistence='p' AND NOT c.relispartition
             AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity
             AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_inherits
                 WHERE inhrelid=c.oid OR inhparent=c.oid)
             AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_rewrite WHERE ev_class=c.oid)
             AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger WHERE tgrelid=c.oid)
             AND pg_catalog.to_regclass($1)=c.oid
             FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname='qadra_ca' AND c.relname=$1",
                &[&table],
            )
            .map_err(port)?
            .ok_or_else(|| invalid("CA identity table is missing"))?;
        if record.get::<_, String>(1) != owner || !record.get::<_, bool>(2) {
            return Err(invalid("CA identity table ownership or structure differs"));
        }
        identity.push(record.get(0));
        columns(client, table, expected)?;
        constraints::validate(client, table)?;
    }
    Ok(identity)
}

fn columns(
    client: &mut impl GenericClient,
    table: &str,
    expected: &[Column],
) -> Result<(), ApplicationError> {
    let rows = client.query(
        "SELECT a.attname::text, pg_catalog.format_type(a.atttypid,a.atttypmod), a.attnotnull,
         (SELECT pg_catalog.pg_get_expr(d.adbin,d.adrelid) FROM pg_catalog.pg_attrdef d
             WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum),
         a.attgenerated='' AND a.attidentity='' AND t.typnamespace='pg_catalog'::pg_catalog.regnamespace
         AND CASE WHEN a.atttypid='pg_catalog.text'::pg_catalog.regtype
             THEN a.attcollation='pg_catalog.\"default\"'::pg_catalog.regcollation
             ELSE a.attcollation=0 END
         FROM pg_catalog.pg_attribute a JOIN pg_catalog.pg_type t ON t.oid=a.atttypid
         WHERE a.attrelid=pg_catalog.to_regclass('qadra_ca.' || $1)
             AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
        &[&table],
    ).map_err(port)?;
    if rows.len() != expected.len()
        || rows.iter().zip(expected).any(|(row, column)| {
            row.get::<_, String>(0) != column.0
                || row.get::<_, String>(1) != column.1
                || row.get::<_, bool>(2) != column.2
                || row.get::<_, Option<String>>(3).as_deref() != column.3
                || !row.get::<_, bool>(4)
        })
    {
        return Err(invalid("CA identity columns or defaults differ"));
    }
    Ok(())
}
