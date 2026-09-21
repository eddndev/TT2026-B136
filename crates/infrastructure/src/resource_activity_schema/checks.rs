use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<String> {
    if table == TABLES[0] {
        return vec!["(initial_revision = 1)".into()];
    }
    let mut result: Vec<String> = [
        r#"((action COLLATE "C") = ANY (ARRAY['link'::text, 'unlink'::text]))"#,
        r#"((status COLLATE "C") = ANY (ARRAY['linked'::text, 'unlinked'::text]))"#,
        r#"((target_kind COLLATE "C") = ANY (ARRAY['hearing'::text, 'deadline'::text]))"#,
        "((reason IS NULL) OR ((char_length(reason) >= 1) AND (char_length(reason) <= 1000) AND (octet_length(reason) <= 4000)))",
        "(octet_length(selection_canonical) = ANY (ARRAY[111, 167]))",
        "((octet_length(submission_canonical) >= 5) AND (octet_length(submission_canonical) <= 1048576))",
        "(submission_digest = sha256(submission_canonical))",
        "(octet_length(capture_canonical) = 49)",
        "(capture_digest = sha256(capture_canonical))",
        "(octet_length(recorded_administration_title) <= 800)",
        "(octet_length(recorded_administration_reference) <= 400)",
        "((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))",
        "((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))",
        "((octet_length(recorded_by_email) >= 1) AND (octet_length(recorded_by_email) <= 1280))",
        "(((recorded_administration_revision IS NULL) AND (recorded_administration_digest IS NULL) AND (recorded_administration_title IS NOT NULL) AND (recorded_administration_reference IS NOT NULL)) OR ((recorded_administration_revision >= 1) AND (recorded_administration_revision <= '4294967295'::bigint) AND (recorded_administration_revision IS NOT NULL) AND (recorded_administration_digest IS NOT NULL) AND (octet_length(recorded_administration_digest) = 32) AND (recorded_administration_title IS NULL) AND (recorded_administration_reference IS NULL)))",
        "(num_nonnulls(act_id, act_revision, act_resource_revision, act_capture_digest) = ANY (ARRAY[0, 4]))",
        "(((target_kind = 'hearing'::text) AND (num_nonnulls(hearing_id, hearing_revision, hearing_submission_digest) = 3) AND (num_nonnulls(deadline_id, deadline_revision, deadline_capture_digest) = 0)) OR ((target_kind = 'deadline'::text) AND (num_nonnulls(deadline_id, deadline_revision, deadline_capture_digest) = 3) AND (num_nonnulls(hearing_id, hearing_revision, hearing_submission_digest) = 0)))",
        "(((action = 'link'::text) AND (status = 'linked'::text) AND (revision = 1) AND (previous_capture_digest IS NULL) AND (reason IS NULL)) OR ((action = 'unlink'::text) AND (status = 'unlinked'::text) AND (revision = 2) AND (previous_capture_digest IS NOT NULL) AND (reason IS NOT NULL)))",
    ].into_iter().map(str::to_owned).collect();
    for field in [
        "revision",
        "resource_revision",
        "act_revision",
        "hearing_revision",
        "deadline_revision",
        "recorded_resource_revision",
    ] {
        result.push(format!(
            "(({field} >= 1) AND ({field} <= '4294967295'::bigint))"
        ));
    }
    result.push(
        "((act_resource_revision >= 2) AND (act_resource_revision <= '4294967295'::bigint))".into(),
    );
    for field in [
        "resource_capture_digest",
        "act_capture_digest",
        "hearing_submission_digest",
        "deadline_capture_digest",
        "previous_capture_digest",
        "recorded_resource_capture_digest",
    ] {
        result.push(format!("(octet_length({field}) = 32)"));
    }
    result
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
