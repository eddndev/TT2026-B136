use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<String> {
    let mut values: Vec<String> = [
        "((account_revision >= 0) AND (auth_generation >= 0) AND (auth_generation <= account_revision))",
        "((octet_length(actor_email) >= 1) AND (octet_length(actor_email) <= 1280))",
        "(octet_length(statement) = 150)",
        "(statement_digest = sha256(statement))",
        "(audit_sequence >= 0)",
    ].into_iter().map(str::to_owned).collect();
    let prefix = if table == TABLES[0] {
        "registered"
    } else {
        "withdrawn"
    };
    values.push(format!(
        "(({prefix}_at_seconds >= 0) AND ({prefix}_at_seconds <= '253402300799'::bigint))"
    ));
    values.push(format!(
        "(({prefix}_at_nanoseconds >= 0) AND ({prefix}_at_nanoseconds <= 999999999))"
    ));
    if table == TABLES[0] {
        for name in ["binding_id", "owner_id", "deployment_id"] {
            values.push(format!(
                "({name} <> '00000000-0000-0000-0000-000000000000'::uuid)"
            ));
        }
        values.extend([
            "((trust_revision >= 1) AND (trust_revision <= '4294967295'::bigint))",
            "(octet_length(root_fingerprint) = 32)",
            "(leaf_fingerprint = sha256(certificate_der))",
            "((octet_length(certificate_der) >= 1) AND (octet_length(certificate_der) <= 16384))",
            "(octet_length(signature) = 384)",
            "((octet_length(certificate_subject) >= 1) AND (octet_length(certificate_subject) <= 65536))",
            "((octet_length(certificate_issuer) >= 1) AND (octet_length(certificate_issuer) <= 65536))",
            r#"((certificate_serial COLLATE "C") ~ '^[0-9A-F]{1,40}$'::text)"#,
            "(certificate_not_before <= certificate_not_after)",
            "((valid_from <= checked_at) AND (checked_at <= valid_until))",
            "((registered_at_seconds >= checked_at) AND (registered_at_seconds <= valid_until))",
        ].into_iter().map(str::to_owned));
    }
    values
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
