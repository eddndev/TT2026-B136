use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

pub(super) fn expected(table: &str) -> Vec<String> {
    let values: &[&str] = if table == TABLES[1] {
        &["singleton", "(first_audit_sequence >= 1)"]
    } else {
        &[
            "((document_version >= 1) AND (document_version <= '4294967295'::bigint))",
            "((octet_length(manifest) >= 1) AND (octet_length(manifest) <= 65536))",
            "(manifest_sha256 = sha256(manifest))",
            "(octet_length(document_sha256) = 32)",
            "((octet_length(manifest_token) >= 1) AND (octet_length(manifest_token) <= 65536))",
            "((octet_length(tsa_crl_pem) >= 1) AND (octet_length(tsa_crl_pem) <= 1500000))",
        ]
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
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
        let mut wanted = expected(table);
        actual.sort();
        wanted.sort();
        if actual != wanted || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
