use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<(String, String)> {
    if table != TABLES[0] {
        return Vec::new();
    }
    [
        ("result_initial", "(result_revision = 1)"),
        ("deadline_initial", "(deadline_revision = 1)"),
        ("event_positive", "(source_event_sequence > 0)"),
        ("audit_nonnegative", "(audit_sequence >= 0)"),
        ("actor_email", "case_administration_text_valid(actor_email, 320, false)"),
        ("actor_role", r#"((actor_role COLLATE "C") = ANY (ARRAY['owner'::text, 'litigator'::text]))"#),
        ("review_size", "((octet_length(review_canonical) >= 5) AND (octet_length(review_canonical) <= 1048576) AND (SUBSTRING(review_canonical FROM 1 FOR 5) = convert_to('HRDL1'::text, 'UTF8'::name)))"),
        ("review_hash", "(review_digest = sha256(review_canonical))"),
        ("capture_size", "((octet_length(capture_canonical) >= 457) AND (octet_length(capture_canonical) <= 5808) AND (SUBSTRING(capture_canonical FROM 1 FOR 5) = convert_to('HRDC1'::text, 'UTF8'::name)))"),
        ("capture_hash", "(capture_digest = sha256(capture_canonical))"),
    ]
    .into_iter()
    .map(|(name, expression)| (format!("hearing_derived_deadline_{name}"), expression.to_owned()))
    .collect()
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client
            .query(
                "SELECT c.conname::text,pg_get_expr(c.conbin,c.conrelid),
                c.convalidated AND NOT c.connoinherit
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.contype='c'",
                &[&table],
            )
            .map_err(port)?;
        let mut actual: Vec<(String, String)> =
            rows.iter().map(|row| (row.get(0), row.get(1))).collect();
        let mut expected = expected(table);
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(2)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
