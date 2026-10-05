use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<(String, String)> {
    let (prefix, expressions): (&str, &[(&str, &str)]) = match table {
        "case_measure_operations" => ("measure_operation", &[
            ("family", r#"((family COLLATE "C") = 'g1'::text)"#),
            ("digest", "(octet_length(owner_digest) = 32)"),
            ("sequence", "(audit_sequence >= 0)"),
        ]),
        "case_measure_decisions" => ("measure_decision", &[
            ("values_size", "(((octet_length(values_canonical) >= 80) AND (octet_length(values_canonical) <= 16076)) AND (SUBSTRING(values_canonical FROM 1 FOR 6) = convert_to('MDVAL1'::text, 'UTF8'::name)))"),
            ("values_view", "((jsonb_typeof(values_view) = 'object'::text) AND (octet_length((values_view)::text) <= 32768))"),
            ("values_hash", "(values_digest = sha256(values_canonical))"),
            ("outcome_size", "(((octet_length(outcome_canonical) >= 11) AND (octet_length(outcome_canonical) <= 645450)) AND (SUBSTRING(outcome_canonical FROM 1 FOR 5) = convert_to('MEFX1'::text, 'UTF8'::name)))"),
            ("outcome_view", "((jsonb_typeof(outcome_view) = 'object'::text) AND (octet_length((outcome_view)::text) <= 1048576))"),
            ("outcome_hash", "(outcome_digest = sha256(outcome_canonical))"),
            ("administration_range", "((observed_administration_revision >= 1) AND (observed_administration_revision <= '4294967295'::bigint))"),
            ("stage_range", "((observed_stage_revision >= 1) AND (observed_stage_revision <= '4294967295'::bigint))"),
            ("context_size", "(octet_length(observed_context_digest) = 32)"),
            ("format", r#"((support_format COLLATE "C") = ANY (ARRAY['pdf'::text, 'docx'::text]))"#),
            ("policy", r#"((support_policy COLLATE "C") = 'pdf_docx_v1'::text)"#),
            ("email", "case_administration_text_valid(recorded_by_email, 320, false)"),
            ("role", r#"((recorded_by_role COLLATE "C") = ANY (ARRAY['owner'::text, 'litigator'::text]))"#),
            ("seconds", "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))"),
            ("nanoseconds", "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))"),
            ("submission_size", "(octet_length(submission_digest) = 32)"),
            ("review_size", "(octet_length(review_digest) = 32)"),
            ("capture_size", "(octet_length(decision_digest) = 32)"),
            ("group_size", "(octet_length(group_digest) = 32)"),
            ("anchor_shape", r#"((((anchor_kind COLLATE "C") = 'none'::text) AND (num_nonnulls(anchor_hearing_id, anchor_revision, anchor_values_digest, anchor_submission_digest) = 0)) OR (((anchor_kind COLLATE "C") = 'initial'::text) AND (num_nonnulls(anchor_hearing_id, anchor_revision, anchor_values_digest, anchor_submission_digest) = 4) AND (anchor_revision >= 1) AND (anchor_revision <= '4294967295'::bigint) AND (octet_length(anchor_values_digest) = 32) AND (octet_length(anchor_submission_digest) = 32)))"#),
        ]),
        "case_measures" => ("measure_root", &[("initial", "(initial_revision = 1)")]),
        "case_measure_revisions" => ("measure_revision", &[
            ("range", "((revision >= 1) AND (revision <= '4294967295'::bigint))"),
            ("family", r#"((family COLLATE "C") = 'm1'::text)"#),
            ("action", r#"((action COLLATE "C") = ANY (ARRAY['impose'::text, 'confirm'::text, 'modify'::text, 'revoke'::text, 'cease'::text, 'substitute_out'::text, 'substitute_in'::text]))"#),
            ("values_size", "(((octet_length(values_canonical) >= 91) AND (octet_length(values_canonical) <= 20113)) AND (SUBSTRING(values_canonical FROM 1 FOR 5) = convert_to('MEAS1'::text, 'UTF8'::name)))"),
            ("values_view", "((jsonb_typeof(values_view) = 'object'::text) AND (octet_length((values_view)::text) <= 32768))"),
            ("values_hash", "(values_digest = sha256(values_canonical))"),
            ("capture_size", "(octet_length(capture_digest) = 32)"),
            ("subject_revision", "((subject_revision >= 1) AND (subject_revision <= '4294967295'::bigint))"),
            ("subject_digest", "(octet_length(subject_values_digest) = 32)"),
            ("supervisor_pair", "((supervisor_id IS NULL) = (supervisor_revision IS NULL))"),
            ("supervisor_revision", "((supervisor_revision >= 1) AND (supervisor_revision <= '4294967295'::bigint))"),
        ]),
        _ => ("", &[]),
    };
    expressions
        .iter()
        .map(|(name, expression)| (format!("{prefix}_{name}"), (*expression).to_owned()))
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
