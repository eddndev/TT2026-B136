//! Exact CHECK expressions, including the nullable act and administration shapes.
use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [&'static str] {
    match table {
        "case_procedural_resources" => &["(initial_revision = 1)"],
        "case_procedural_resource_acts" => &[
            "((initial_resource_revision >= 2) AND (initial_resource_revision <= '4294967295'::bigint))",
        ],
        "case_procedural_resource_revisions" => &[
            "((revision >= 1) AND (revision <= '4294967295'::bigint))",
            r#"((action COLLATE "C") = ANY (ARRAY['register'::text, 'correct'::text, 'record_act'::text, 'correct_act'::text, 'archive'::text, 'reactivate'::text]))"#,
            r#"((status COLLATE "C") = ANY (ARRAY['active'::text, 'archived'::text]))"#,
            r#"((kind COLLATE "C") = ANY (ARRAY['revocation'::text, 'appeal'::text]))"#,
            "((reason IS NULL) OR ((char_length(reason) >= 1) AND (char_length(reason) <= 1000) AND (octet_length(reason) <= 4000)))",
            "((octet_length(values_canonical) >= 5) AND (octet_length(values_canonical) <= 262144))",
            "(octet_length((values_view)::text) <= 524288)",
            "(values_digest = sha256(values_canonical))",
            "((octet_length(sources_canonical) >= 5) AND (octet_length(sources_canonical) <= 524288))",
            "(sources_digest = sha256(sources_canonical))",
            "((jsonb_typeof(supports_view) = 'array'::text) AND (jsonb_array_length(supports_view) <= 2) AND (octet_length((supports_view)::text) <= 16384))",
            "((octet_length(submission_canonical) >= 5) AND (octet_length(submission_canonical) <= 1048576))",
            "(submission_digest = sha256(submission_canonical))",
            "(octet_length(capture_canonical) = 57)",
            "(capture_digest = sha256(capture_canonical))",
            "(octet_length(previous_capture_digest) = 32)",
            "((act_revision >= 1) AND (act_revision <= '4294967295'::bigint))",
            "((octet_length(act_values_canonical) >= 5) AND (octet_length(act_values_canonical) <= 32768))",
            "(octet_length((act_values_view)::text) <= 65536)",
            "((jsonb_typeof(act_supports_view) = 'array'::text) AND (jsonb_array_length(act_supports_view) <= 2) AND (octet_length((act_supports_view)::text) <= 16384))",
            "((act_previous_resource_revision >= 2) AND (act_previous_resource_revision <= '4294967295'::bigint))",
            "(octet_length(act_previous_capture_digest) = 32)",
            "(octet_length(recorded_administration_title) <= 800)",
            "(octet_length(recorded_administration_reference) <= 400)",
            "((recorded_stage_revision >= 1) AND (recorded_stage_revision <= '4294967295'::bigint))",
            "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))",
            "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))",
            "((octet_length(recorded_by_email) >= 1) AND (octet_length(recorded_by_email) <= 1280))",
            "(((recorded_administration_revision IS NULL) AND (recorded_administration_digest IS NULL) AND (recorded_administration_title IS NOT NULL) AND (recorded_administration_reference IS NOT NULL)) OR ((recorded_administration_revision >= 1) AND (recorded_administration_revision <= '4294967295'::bigint) AND (recorded_administration_revision IS NOT NULL) AND (recorded_administration_digest IS NOT NULL) AND (octet_length(recorded_administration_digest) = 32) AND (recorded_administration_title IS NULL) AND (recorded_administration_reference IS NULL)))",
            "(((action = 'register'::text) AND (revision = 1) AND (previous_capture_digest IS NULL) AND (reason IS NULL)) OR ((action <> 'register'::text) AND (revision > 1) AND (previous_capture_digest IS NOT NULL) AND (((action = 'record_act'::text) AND (reason IS NULL)) OR ((action <> 'record_act'::text) AND (reason IS NOT NULL)))))",
            "((action = 'archive'::text) = (status = 'archived'::text))",
            "(((action = ANY (ARRAY['record_act'::text, 'correct_act'::text])) AND (act_id IS NOT NULL) AND (act_revision IS NOT NULL) AND (act_values_canonical IS NOT NULL) AND (act_values_view IS NOT NULL) AND (act_supports_view IS NOT NULL) AND (((action = 'record_act'::text) AND (act_revision = 1) AND (act_previous_resource_revision IS NULL) AND (act_previous_capture_digest IS NULL)) OR ((action = 'correct_act'::text) AND (act_revision > 1) AND (act_previous_resource_revision IS NOT NULL) AND (act_previous_capture_digest IS NOT NULL)))) OR ((action <> ALL (ARRAY['record_act'::text, 'correct_act'::text])) AND (act_id IS NULL) AND (act_revision IS NULL) AND (act_values_canonical IS NULL) AND (act_values_view IS NULL) AND (act_supports_view IS NULL) AND (act_previous_resource_revision IS NULL) AND (act_previous_capture_digest IS NULL)))",
        ],
        _ => &[],
    }
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
        let mut expected: Vec<String> = expected(table)
            .iter()
            .map(|value| (*value).into())
            .collect();
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
