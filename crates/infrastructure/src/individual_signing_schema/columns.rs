use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

fn expected(_: &str) -> &'static [(&'static str, &'static str)] {
    &[
        ("operation_id", "uuid"),
        ("case_id", "uuid"),
        ("document_id", "uuid"),
        ("document_version", "bigint"),
        ("signer_id", "uuid"),
        ("binding_id", "uuid"),
        ("deployment_id", "uuid"),
        ("trust_revision", "bigint"),
        ("root_fingerprint", "bytea"),
        ("leaf_fingerprint", "bytea"),
        ("account_revision", "bigint"),
        ("auth_generation", "bigint"),
        ("actor_email", "text"),
        ("consent", "bytea"),
        ("consent_digest", "bytea"),
        ("content_digest", "bytea"),
        ("pdf_signature", "bytea"),
        ("consent_signature", "bytea"),
        ("checked_at", "bigint"),
        ("valid_from", "bigint"),
        ("valid_until", "bigint"),
        ("accepted_at_seconds", "bigint"),
        ("accepted_at_nanoseconds", "integer"),
        ("audit_sequence", "bigint"),
    ]
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client.query(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation ELSE a.attcollation=0 END,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum", &[&table],
        ).map_err(port)?;
        if rows.len() != expected(table).len()
            || rows.iter().zip(expected(table)).any(|(row, (name, kind))| {
                row.get::<_, String>(0) != *name
                    || row.get::<_, String>(1) != *kind
                    || !row.get::<_, bool>(2)
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
