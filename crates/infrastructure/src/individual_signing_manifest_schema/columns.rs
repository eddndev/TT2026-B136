use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

fn expected(table: &str) -> &'static [(&'static str, &'static str)] {
    if table == TABLES[1] {
        return &[("singleton", "boolean"), ("first_audit_sequence", "bigint")];
    }
    &[
        ("operation_id", "uuid"),
        ("document_id", "uuid"),
        ("document_version", "bigint"),
        ("manifest", "bytea"),
        ("manifest_sha256", "bytea"),
        ("document_sha256", "bytea"),
        ("manifest_token", "bytea"),
        ("tsa_crl_pem", "bytea"),
    ]
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}

/// Columns the runtime inserts: every manifest column and no cutoff column.
pub(super) fn insertable(table: &str) -> Vec<&'static str> {
    if table == TABLES[1] {
        return Vec::new();
    }
    names(table)
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client
            .query(
                "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,a.attcollation=0,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
                &[&table],
            )
            .map_err(port)?;
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
