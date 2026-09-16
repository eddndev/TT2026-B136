//! Exact catalog expressions for migrations/0013_judicial_calendar_tables.sql.
use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;
const CHECKS: &[(&str, &str)] = &[
    (
        "judicial_calendar_action",
        r#"((action COLLATE "C") = ANY (ARRAY['publish'::text, 'replace'::text, 'retire'::text]))"#,
    ),
    (
        "judicial_calendar_reason",
        r#"(((action = 'publish'::text) AND (reason IS NULL)) OR ((action <> 'publish'::text) AND case_administration_text_valid(reason, 1000, true)))"#,
    ),
    (
        "judicial_calendar_actor_email",
        r#"case_administration_text_valid(recorded_by_email, 320, false)"#,
    ),
    (
        "judicial_calendar_values_hash",
        r#"(values_digest = sha256(values_canonical))"#,
    ),
    (
        "judicial_calendar_values_size",
        r#"((octet_length(values_canonical) >= 99) AND (octet_length(values_canonical) <= 191910))"#,
    ),
    (
        "judicial_calendar_revision_range",
        r#"((revision >= 1) AND (revision <= '4294967295'::bigint))"#,
    ),
    (
        "judicial_calendar_submission_hash",
        r#"(submission_digest = sha256(submission_canonical))"#,
    ),
    (
        "judicial_calendar_submission_size",
        r#"((octet_length(submission_canonical) >= 91) AND (octet_length(submission_canonical) <= 4095))"#,
    ),
    (
        "judicial_calendar_initial_revision",
        r#"(initial_revision = 1)"#,
    ),
    (
        "judicial_calendar_receipt_projection",
        r#"((recorded_by = ((submission_view ->> 'actor_id'::text))::uuid) AND (operation_id = ((submission_view ->> 'operation_id'::text))::uuid) AND (calendar_id = ((submission_view ->> 'calendar_id'::text))::uuid) AND (action = (submission_view ->> 'action'::text)) AND ((revision - 1) = ((submission_view ->> 'expected_revision'::text))::bigint) AND (values_digest = decode((submission_view ->> 'values_digest'::text), 'hex'::text)) AND (NOT ((reason COLLATE "C") IS DISTINCT FROM ((submission_view ->> 'reason'::text) COLLATE "C"))))"#,
    ),
    (
        "judicial_calendar_recorded_time_range",
        r#"((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))"#,
    ),
    (
        "judicial_calendar_recorded_nanos_range",
        r#"((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))"#,
    ),
];
const GENERATED: &[(&str, &str)] = &[
    (
        "status",
        r#"
CASE action
    WHEN 'retire'::text THEN 'retired'::text
    ELSE 'published'::text
END"#,
    ),
    (
        "values_view",
        r#"judicial_calendar_values(values_canonical)"#,
    ),
    (
        "submission_view",
        r#"judicial_calendar_submission(submission_canonical)"#,
    ),
];
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let checks=client.query("SELECT conname::text,pg_get_expr(conbin,conrelid),convalidated,connoinherit FROM pg_constraint WHERE conrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND contype='c'", &[&&TABLES[..]]).map_err(port)?;
    if checks.len() != CHECKS.len()
        || CHECKS.iter().any(|(name, expression)| {
            !checks.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && normalize(&r.get::<_, String>(1)) == normalize(expression)
                    && r.get::<_, bool>(2)
                    && !r.get::<_, bool>(3)
            })
        })
    {
        return Err(incomplete());
    }
    let generated=client.query("SELECT a.attname::text,pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum WHERE d.adrelid='judicial_calendar_revisions'::regclass", &[]).map_err(port)?;
    if generated.len() != GENERATED.len()
        || GENERATED.iter().any(|(name, expression)| {
            !generated.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && normalize(&r.get::<_, String>(1)) == normalize(expression)
            })
        })
    {
        return Err(incomplete());
    }
    let defaults=client.query("SELECT a.attname::text,pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum WHERE d.adrelid='judicial_calendars'::regclass", &[]).map_err(port)?;
    if defaults.len() != 1
        || defaults[0].get::<_, String>(0) != "initial_revision"
        || defaults[0].get::<_, String>(1) != "1"
    {
        return Err(incomplete());
    }
    Ok(())
}
fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
