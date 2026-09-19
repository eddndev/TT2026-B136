//! Exact catalog expressions for the versioned deadline table contract.
use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;
const CHECKS: &[(&str, &str, &str)] = &[
    (
        "case_deadlines",
        "deadline_initial_revision",
        r#"(initial_revision = 1)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_action",
        r#"(((action COLLATE "C") = ANY (ARRAY['register'::text, 'correct'::text, 'set_attention'::text, 'retire'::text, 'reevaluate'::text])) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_actor_email",
        r#"(((recorded_by_email IS NULL) OR case_administration_text_valid(recorded_by_email, 320, false)) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_administration_hash",
        r#"(observed_administration_digest = sha256(observed_administration_canonical))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_administration_size",
        r#"((octet_length(observed_administration_canonical) >= 17) AND (octet_length(observed_administration_canonical) <= 16384) AND (SUBSTRING(observed_administration_canonical FROM 1 FOR 5) = convert_to('CADM1'::text, 'UTF8'::name)))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_attention",
        r#"deadline_attention_valid(attention)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_calendar_shape",
        r#"(((calendar_id IS NULL) AND (calendar_revision IS NULL) AND (calendar_head_revision IS NULL)) OR ((calendar_id IS NOT NULL) AND (calendar_revision IS NOT NULL) AND (calendar_head_revision IS NOT NULL) AND ((calendar_revision >= 1) AND (calendar_revision <= '4294967295'::bigint)) AND ((calendar_head_revision >= calendar_revision) AND (calendar_head_revision <= '4294967295'::bigint))))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_capture_hash",
        r#"(capture_digest = sha256(capture_canonical))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_capture_size",
        r#"((((SUBSTRING(capture_canonical FROM 1 FOR 5) = convert_to('DLST1'::text, 'UTF8'::name)) AND ((octet_length(capture_canonical) >= 5) AND (octet_length(capture_canonical) <= 524288))) OR ((SUBSTRING(capture_canonical FROM 1 FOR 5) = convert_to('DLST2'::text, 'UTF8'::name)) AND ((octet_length(capture_canonical) >= 5) AND (octet_length(capture_canonical) <= 524410)))) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_due_shape",
        r#"(((due_at_seconds IS NULL) AND (due_at_nanoseconds IS NULL)) OR ((due_at_seconds IS NOT NULL) AND (due_at_nanoseconds IS NOT NULL) AND ((due_at_seconds >= '-62135596800'::bigint) AND (due_at_seconds <= '253402300799'::bigint)) AND ((due_at_nanoseconds >= 0) AND (due_at_nanoseconds <= 999999999))))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_input_projection",
        r#"((case_id = ((input_view ->> 'case_id'::text))::uuid) AND (NOT (source_kind IS DISTINCT FROM (input_view ->> 'source_kind'::text))) AND (NOT (source_id IS DISTINCT FROM ((input_view ->> 'source_id'::text))::uuid)) AND (NOT (source_revision IS DISTINCT FROM ((input_view ->> 'source_revision'::text))::bigint)) AND (NOT (source_hearing_id IS DISTINCT FROM ((input_view ->> 'source_hearing_id'::text))::uuid)) AND (NOT (source_parent_resolution_id IS DISTINCT FROM ((input_view ->> 'source_parent_resolution_id'::text))::uuid)) AND (NOT (source_parent_resolution_revision IS DISTINCT FROM ((input_view ->> 'source_parent_resolution_revision'::text))::bigint)) AND (NOT (calendar_id IS DISTINCT FROM ((input_view ->> 'calendar_id'::text))::uuid)) AND (NOT (calendar_revision IS DISTINCT FROM ((input_view ->> 'calendar_revision'::text))::bigint)))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_input_size",
        r#"((octet_length(input_canonical) >= 48) AND (octet_length(input_canonical) <= 98897))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_reason",
        r#"(((action = 'register'::text) AND (reason IS NULL)) OR ((action <> 'register'::text) AND (reason IS NOT NULL) AND case_administration_text_valid(reason, 1000, true)))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_receipt_projection",
        r#"(((operation_id = ((submission_view ->> 'operation_id'::text))::uuid) AND (deadline_id = ((submission_view ->> 'deadline_id'::text))::uuid) AND (case_id = ((submission_view ->> 'case_id'::text))::uuid) AND ((action COLLATE "C") = ((submission_view ->> 'action'::text) COLLATE "C")) AND ((revision - 1) = ((submission_view ->> 'expected_revision'::text))::bigint) AND (review_digest = decode((submission_view ->> 'review_digest'::text), 'hex'::text)) AND (NOT ((reason COLLATE "C") IS DISTINCT FROM ((submission_view ->> 'reason'::text) COLLATE "C"))) AND
CASE SUBSTRING(submission_canonical FROM 1 FOR 5)
    WHEN convert_to('DLTX1'::text, 'UTF8'::name) THEN (recorded_by = ((submission_view ->> 'actor_id'::text))::uuid)
    WHEN convert_to('DLTX2'::text, 'UTF8'::name) THEN
    CASE ((submission_view -> 'author'::text) ->> 'kind'::text)
        WHEN 'user'::text THEN ((recorded_by = (((submission_view -> 'author'::text) ->> 'id'::text))::uuid) AND ((recorded_by_email COLLATE "C") = (((submission_view -> 'author'::text) ->> 'email'::text) COLLATE "C")))
        WHEN 'technical'::text THEN ((recorded_by IS NULL) AND (recorded_by_email IS NULL))
        ELSE false
    END
    ELSE false
END) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_recorded_nanos",
        r#"((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_recorded_seconds",
        r#"((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_responsible_email",
        r#"case_administration_text_valid(responsible_email, 320, false)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_responsible_role",
        r#"((responsible_role COLLATE "C") = ANY (ARRAY['owner'::text, 'litigator'::text, 'paralegal'::text]))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_result_size",
        r#"((octet_length(result_canonical) >= 5) AND (octet_length(result_canonical) <= 3000000) AND (SUBSTRING(result_canonical FROM 1 FOR 5) = convert_to('DRES1'::text, 'UTF8'::name)))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_review_hash",
        r#"(review_digest = sha256(review_canonical))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_review_size",
        r#"((((SUBSTRING(review_canonical FROM 1 FOR 5) = convert_to('DLRV1'::text, 'UTF8'::name)) AND ((octet_length(review_canonical) >= 5) AND (octet_length(review_canonical) <= 524288))) OR ((SUBSTRING(review_canonical FROM 1 FOR 5) = convert_to('DLRV2'::text, 'UTF8'::name)) AND ((octet_length(review_canonical) >= 5) AND (octet_length(review_canonical) <= 524341)))) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_revision_range",
        r#"((revision >= 1) AND (revision <= '4294967295'::bigint))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_source_shape",
        r#"(((source_kind IS NULL) AND (num_nonnulls(source_id, source_revision, source_head_revision, source_hearing_id, source_parent_resolution_id, source_parent_resolution_revision, source_head_parent_resolution_revision) = 0)) OR ((source_kind IS NOT NULL) AND (source_kind = ANY (ARRAY['resolution'::text, 'notification'::text, 'hearing_result'::text])) AND (source_id IS NOT NULL) AND (source_revision IS NOT NULL) AND (source_head_revision IS NOT NULL) AND ((source_revision >= 1) AND (source_revision <= '4294967295'::bigint)) AND ((source_head_revision >= source_revision) AND (source_head_revision <= '4294967295'::bigint)) AND (((source_kind = 'resolution'::text) AND (num_nonnulls(source_hearing_id, source_parent_resolution_id, source_parent_resolution_revision, source_head_parent_resolution_revision) = 0)) OR ((source_kind = 'notification'::text) AND (source_hearing_id IS NULL) AND (source_parent_resolution_id IS NOT NULL) AND (source_parent_resolution_revision IS NOT NULL) AND (source_head_parent_resolution_revision IS NOT NULL) AND ((source_parent_resolution_revision >= 1) AND (source_parent_resolution_revision <= '4294967295'::bigint)) AND ((source_head_parent_resolution_revision >= 1) AND (source_head_parent_resolution_revision <= '4294967295'::bigint))) OR ((source_kind = 'hearing_result'::text) AND (source_hearing_id IS NOT NULL) AND (num_nonnulls(source_parent_resolution_id, source_parent_resolution_revision, source_head_parent_resolution_revision) = 0)))))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_submission_hash",
        r#"(submission_digest = sha256(submission_canonical))"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_submission_size",
        r#"((((SUBSTRING(submission_canonical FROM 1 FOR 5) = convert_to('DLTX1'::text, 'UTF8'::name)) AND ((octet_length(submission_canonical) >= 107) AND (octet_length(submission_canonical) <= 4115))) OR ((SUBSTRING(submission_canonical FROM 1 FOR 5) = convert_to('DLTX2'::text, 'UTF8'::name)) AND ((octet_length(submission_canonical) >= 151) AND (octet_length(submission_canonical) <= 5502)))) IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_title",
        r#"case_administration_text_valid(title, 200, false)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_recorded_author",
        r#"(
CASE SUBSTRING(submission_canonical FROM 1 FOR 5)
    WHEN convert_to('DLTX1'::text, 'UTF8'::name) THEN ((recorded_by IS NOT NULL) AND (recorded_by_email IS NOT NULL) AND (action <> 'reevaluate'::text))
    WHEN convert_to('DLTX2'::text, 'UTF8'::name) THEN
    CASE ((submission_view -> 'author'::text) ->> 'kind'::text)
        WHEN 'user'::text THEN ((recorded_by IS NOT NULL) AND (recorded_by_email IS NOT NULL) AND (action <> 'reevaluate'::text) AND ((submission_view -> 'cause'::text) = 'null'::jsonb))
        WHEN 'technical'::text THEN ((recorded_by IS NULL) AND (recorded_by_email IS NULL) AND (action = 'reevaluate'::text) AND (((submission_view -> 'author'::text) ->> 'service'::text) = 'deadline_reevaluator'::text) AND ((((submission_view -> 'author'::text) ->> 'policy_version'::text))::bigint = 1) AND (((submission_view -> 'cause'::text) ->> 'kind'::text) = ANY (ARRAY['source_event'::text, 'legacy_bootstrap'::text])))
        ELSE false
    END
    ELSE false
END IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_tracking_commitments",
        r#"(
CASE SUBSTRING(submission_canonical FROM 1 FOR 5)
    WHEN convert_to('DLTX1'::text, 'UTF8'::name) THEN ((tracking_canonical IS NULL) AND (observations_canonical IS NULL))
    WHEN convert_to('DLTX2'::text, 'UTF8'::name) THEN ((tracking_canonical IS NOT NULL) AND (observations_canonical IS NOT NULL) AND deadline_tracking_consistent(tracking_canonical, observations_canonical) AND (case_id = ((deadline_observations(observations_canonical) ->> 'case_id'::text))::uuid) AND (sha256(observations_canonical) = decode((deadline_tracking(tracking_canonical) ->> 'observations_digest'::text), 'hex'::text)) AND (sha256(observations_canonical) = decode((submission_view ->> 'observations_digest'::text), 'hex'::text)) AND (SUBSTRING(capture_canonical FROM ((octet_length(capture_canonical) - octet_length(tracking_canonical)) + 1)) = tracking_canonical) AND (SUBSTRING(review_canonical FROM ((octet_length(review_canonical) - (37 + (2 * get_byte(tracking_canonical, 4)))) + 1)) = SUBSTRING(tracking_canonical FROM 1 FOR (37 + (2 * get_byte(tracking_canonical, 4))))))
    ELSE false
END IS TRUE)"#,
    ),
    (
        "case_deadline_revisions",
        "deadline_tracking_shape",
        r#"(
CASE SUBSTRING(submission_canonical FROM 1 FOR 5)
    WHEN convert_to('DLTX1'::text, 'UTF8'::name) THEN ((SUBSTRING(review_canonical FROM 1 FOR 5) = convert_to('DLRV1'::text, 'UTF8'::name)) AND (SUBSTRING(capture_canonical FROM 1 FOR 5) = convert_to('DLST1'::text, 'UTF8'::name)) AND (tracking_canonical IS NULL) AND (observations_canonical IS NULL) AND (tracking_administration_revision IS NULL) AND (cause_event_sequence IS NULL))
    WHEN convert_to('DLTX2'::text, 'UTF8'::name) THEN ((SUBSTRING(review_canonical FROM 1 FOR 5) = convert_to('DLRV2'::text, 'UTF8'::name)) AND (SUBSTRING(capture_canonical FROM 1 FOR 5) = convert_to('DLST2'::text, 'UTF8'::name)) AND (tracking_canonical IS NOT NULL) AND (observations_canonical IS NOT NULL) AND ((octet_length(tracking_canonical) >= 102) AND (octet_length(tracking_canonical) <= 122)) AND ((octet_length(observations_canonical) >= 111) AND (octet_length(observations_canonical) <= 446)))
    ELSE false
END IS TRUE)"#,
    ),
];
const DEFAULTS: &[(&str, &str, &str)] = &[
    ("case_deadlines", "initial_revision", r#"1"#),
    (
        "case_deadline_revisions",
        "input_view",
        r#"deadline_input_selection(input_canonical)"#,
    ),
    (
        "case_deadline_revisions",
        "status",
        r#"
CASE action
    WHEN 'retire'::text THEN 'retired'::text
    ELSE 'active'::text
END"#,
    ),
    (
        "case_deadline_revisions",
        "submission_view",
        r#"deadline_submission(submission_canonical)"#,
    ),
    (
        "case_deadline_revisions",
        "source_fact_family",
        r#"
CASE
    WHEN (source_kind = ANY (ARRAY['resolution'::text, 'notification'::text])) THEN source_kind
    ELSE NULL::text
END"#,
    ),
    (
        "case_deadline_revisions",
        "source_fact_id",
        r#"
CASE
    WHEN (source_kind = ANY (ARRAY['resolution'::text, 'notification'::text])) THEN source_id
    ELSE NULL::uuid
END"#,
    ),
    (
        "case_deadline_revisions",
        "source_result_id",
        r#"
CASE
    WHEN (source_kind = 'hearing_result'::text) THEN source_id
    ELSE NULL::uuid
END"#,
    ),
    (
        "case_deadline_revisions",
        "source_parent_family",
        r#"
CASE
    WHEN (source_kind = 'notification'::text) THEN 'resolution'::text
    ELSE NULL::text
END"#,
    ),
    (
        "case_deadline_revisions",
        "cause_event_sequence",
        r#"((((deadline_submission(submission_canonical) -> 'cause'::text) -> 'event'::text) ->> 'sequence'::text))::bigint"#,
    ),
    (
        "case_deadline_revisions",
        "tracking_administration_revision",
        r#"((deadline_tracking(tracking_canonical) ->> 'administration_revision'::text))::bigint"#,
    ),
];
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let checks=client.query("SELECT c.relname::text,k.conname::text,pg_get_expr(k.conbin,k.conrelid),k.convalidated,k.connoinherit FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid WHERE k.conrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND k.contype='c'", &[&&TABLES[..]]).map_err(port)?;
    if checks.len() != CHECKS.len()
        || CHECKS.iter().any(|(table, name, expression)| {
            !checks.iter().any(|r| {
                r.get::<_, String>(0) == *table
                    && r.get::<_, String>(1) == *name
                    && normalize(&r.get::<_, String>(2)) == normalize(expression)
                    && r.get::<_, bool>(3)
                    && !r.get::<_, bool>(4)
            })
        })
    {
        return Err(incomplete());
    }
    let defaults=client.query("SELECT c.relname::text,a.attname::text,pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d JOIN pg_class c ON c.oid=d.adrelid JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum WHERE d.adrelid IN (SELECT t::regclass FROM unnest($1::text[]) t)", &[&&TABLES[..]]).map_err(port)?;
    if defaults.len() != DEFAULTS.len()
        || DEFAULTS.iter().any(|(table, name, expression)| {
            !defaults.iter().any(|r| {
                r.get::<_, String>(0) == *table
                    && r.get::<_, String>(1) == *name
                    && normalize(&r.get::<_, String>(2)) == normalize(expression)
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}
fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
