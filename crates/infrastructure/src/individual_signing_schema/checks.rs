use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

pub(super) fn expected(_: &str) -> Vec<String> {
    let mut values: Vec<String> = [
        "((account_revision >= 0) AND (auth_generation >= 0) AND (auth_generation <= account_revision))",
        "((octet_length(actor_email) >= 1) AND (octet_length(actor_email) <= 1280))",
        "((document_version >= 1) AND (document_version <= '4294967295'::bigint))",
        "((trust_revision >= 1) AND (trust_revision <= '4294967295'::bigint))",
        "(octet_length(root_fingerprint) = 32)",
        "(octet_length(leaf_fingerprint) = 32)",
        "(octet_length(consent) = 258)",
        "(consent_digest = sha256(consent))",
        "(octet_length(content_digest) = 32)",
        "(octet_length(pdf_signature) = 384)",
        "(octet_length(consent_signature) = 384)",
        "((valid_from <= checked_at) AND (checked_at <= valid_until))",
        "((accepted_at_seconds >= checked_at) AND (accepted_at_seconds <= valid_until))",
        "((accepted_at_seconds >= 0) AND (accepted_at_seconds <= '253402300799'::bigint))",
        "((accepted_at_nanoseconds >= 0) AND (accepted_at_nanoseconds <= 999999999))",
        "(audit_sequence >= 0)",
    ].into_iter().map(str::to_owned).collect();
    for name in [
        "operation_id",
        "case_id",
        "document_id",
        "signer_id",
        "binding_id",
        "deployment_id",
    ] {
        values.push(format!(
            "({name} <> '00000000-0000-0000-0000-000000000000'::uuid)"
        ));
    }
    values
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT pg_get_expr(c.conbin,c.conrelid),c.convalidated AND NOT c.connoinherit
            AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
        FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
        WHERE c.conrelid=$1::text::regclass AND c.contype='c'",
            &[&TABLES[0]],
        )
        .map_err(port)?;
    let mut actual: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
    let mut expected = expected(TABLES[0]);
    actual.sort();
    expected.sort();
    if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
        return Err(incomplete());
    }
    Ok(())
}
