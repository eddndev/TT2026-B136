use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<(String, String)> {
    if table == TABLES[0] {
        return vec![(
            "precautionary_hearing_initial".into(),
            "(initial_revision = 1)".into(),
        )];
    }
    [
        ("revision_range", "((revision >= 1) AND (revision <= '4294967295'::bigint))"),
        ("action", r#"((action COLLATE "C") = ANY (ARRAY['schedule'::text, 'replace'::text, 'cancel'::text]))"#),
        ("predecessor_size", "(octet_length(previous_capture_digest) = 32)"),
        ("reason", "((reason IS NULL) OR case_administration_text_valid(reason, 1000, true))"),
        ("command_shape", "(((action = 'schedule'::text) AND (revision = 1) AND (previous_capture_digest IS NULL) AND (reason IS NULL)) OR ((action = ANY (ARRAY['replace'::text, 'cancel'::text])) AND (revision > 1) AND (previous_capture_digest IS NOT NULL) AND (reason IS NOT NULL)))"),
        ("values_presence", "((action = 'cancel'::text) = (values_canonical IS NULL))"),
        ("view_presence", "((values_canonical IS NULL) = (values_view IS NULL))"),
        ("values_digest_presence", "((values_canonical IS NULL) = (values_digest IS NULL))"),
        ("format_presence", "((values_canonical IS NULL) = (support_format IS NULL))"),
        ("policy_presence", "((values_canonical IS NULL) = (support_policy IS NULL))"),
        ("values_size", "(((octet_length(values_canonical) >= 6) AND (octet_length(values_canonical) <= 16395)) AND (SUBSTRING(values_canonical FROM 1 FOR 6) = convert_to('PHEAR1'::text, 'UTF8'::name)))"),
        ("view_size", "((jsonb_typeof(values_view) = 'object'::text) AND (octet_length((values_view)::text) <= 65536))"),
        ("values_hash", "(values_digest = sha256(values_canonical))"),
        ("administration_range", "((observed_administration_revision >= 1) AND (observed_administration_revision <= '4294967295'::bigint))"),
        ("stage_range", "((observed_stage_revision >= 1) AND (observed_stage_revision <= '4294967295'::bigint))"),
        ("context_size", "(octet_length(observed_context_digest) = 32)"),
        ("format", r#"((support_format COLLATE "C") = ANY (ARRAY['pdf'::text, 'docx'::text]))"#),
        ("policy", r#"((support_policy COLLATE "C") = 'pdf_docx_v1'::text)"#),
        ("actor_email", "case_administration_text_valid(recorded_by_email, 320, false)"),
        ("actor_role", r#"((recorded_by_role COLLATE "C") = ANY (ARRAY['owner'::text, 'litigator'::text]))"#),
        ("seconds", "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))"),
        ("nanoseconds", "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))"),
        ("submission_size", "(octet_length(submission_digest) = 32)"),
        ("review_size", "(octet_length(review_digest) = 32)"),
        ("capture_size", "(octet_length(capture_digest) = 32)"),
        ("audit_nonnegative", "(audit_sequence >= 0)"),
    ]
    .into_iter()
    .map(|(name, expression)| (format!("precautionary_hearing_{name}"), expression.to_owned()))
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
