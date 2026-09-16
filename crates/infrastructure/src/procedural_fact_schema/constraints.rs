//! Catalog expressions for migrations/0014_procedural_facts.sql.
use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

const CHECKS: &[(&str, &str, &str)] = &[
    (
        "case_procedural_fact_revisions",
        "procedural_fact_action",
        r#"((action COLLATE "C") = ANY (ARRAY['record'::text, 'correct'::text, 'withdraw'::text]))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_actor_email",
        r#"case_administration_text_valid(recorded_by_email, 320, false)"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_administration_shape",
        r#"(((recorded_administration_revision IS NULL) AND (recorded_administration_digest IS NULL) AND (recorded_administration_title IS NOT NULL) AND (recorded_administration_reference IS NOT NULL)) OR ((recorded_administration_revision IS NOT NULL) AND ((recorded_administration_revision >= 1) AND (recorded_administration_revision <= '4294967295'::bigint)) AND (recorded_administration_digest IS NOT NULL) AND (octet_length(recorded_administration_digest) = 32) AND (recorded_administration_title IS NULL) AND (recorded_administration_reference IS NULL)))"#,
    ),
    (
        "case_procedural_facts",
        "procedural_fact_family",
        r#"((family COLLATE "C") = ANY (ARRAY['resolution'::text, 'notification'::text]))"#,
    ),
    (
        "case_procedural_facts",
        "procedural_fact_initial_revision",
        r#"(initial_revision = 1)"#,
    ),
    (
        "case_procedural_facts",
        "procedural_fact_parent_shape",
        r#"(((family = 'resolution'::text) AND (parent_resolution_id IS NULL)) OR ((family = 'notification'::text) AND (parent_resolution_id IS NOT NULL)))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_reason",
        r#"(((action = 'record'::text) AND (reason IS NULL)) OR ((action <> 'record'::text) AND (reason IS NOT NULL) AND case_administration_text_valid(reason, 1000, true)))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_receipt_projection",
        r#"((family = (submission_view ->> 'family'::text)) AND (id = ((submission_view ->> 'target_id'::text))::uuid) AND (operation_id = ((submission_view ->> 'operation_id'::text))::uuid) AND (recorded_by = ((submission_view ->> 'actor_id'::text))::uuid) AND (case_id = ((submission_view ->> 'case_id'::text))::uuid) AND (action = (submission_view ->> 'action'::text)) AND ((revision - 1) = ((submission_view ->> 'expected_revision'::text))::bigint) AND (values_digest = decode((submission_view ->> 'values_digest'::text), 'hex'::text)) AND (sources_digest = decode((submission_view ->> 'sources_digest'::text), 'hex'::text)) AND (NOT (reason IS DISTINCT FROM (submission_view ->> 'reason'::text))))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_recorded_nanos_range",
        r#"((recorded_at_nanoseconds >= 0) AND (recorded_at_nanoseconds <= 999999999))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_recorded_time_range",
        r#"((recorded_at_seconds >= '-62135596800'::bigint) AND (recorded_at_seconds <= '253402300799'::bigint))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_revision_range",
        r#"((revision >= 1) AND (revision <= '4294967295'::bigint))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_sources_hash",
        r#"(sources_digest = sha256(sources_canonical))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_sources_size",
        r#"((octet_length(sources_canonical) >= 19) AND (octet_length(sources_canonical) <= 36847))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_submission_hash",
        r#"(submission_digest = sha256(submission_canonical))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_submission_size",
        r#"(((family = 'resolution'::text) AND ((octet_length(submission_canonical) >= 141) AND (octet_length(submission_canonical) <= 4145))) OR ((family = 'notification'::text) AND ((octet_length(submission_canonical) >= 157) AND (octet_length(submission_canonical) <= 4161))))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_values_hash",
        r#"(values_digest = sha256(values_canonical))"#,
    ),
    (
        "case_procedural_fact_revisions",
        "procedural_fact_values_size",
        r#"(((family = 'resolution'::text) AND ((octet_length(values_canonical) >= 27) AND (octet_length(values_canonical) <= 17700))) OR ((family = 'notification'::text) AND ((octet_length(values_canonical) >= 67) AND (octet_length(values_canonical) <= 58671))))"#,
    ),
];
const EXPRESSIONS: &[(&str, &str, &str)] = &[
    (
        "case_procedural_facts",
        "parent_family",
        r#"
CASE
    WHEN (parent_resolution_id IS NULL) THEN NULL::text
    ELSE 'resolution'::text
END"#,
    ),
    ("case_procedural_facts", "initial_revision", r#"1"#),
    (
        "case_procedural_fact_revisions",
        "values_view",
        r#"procedural_fact_values(family, values_canonical)"#,
    ),
    (
        "case_procedural_fact_revisions",
        "sources_view",
        r#"procedural_fact_sources(sources_canonical)"#,
    ),
    (
        "case_procedural_fact_revisions",
        "status",
        r#"
CASE action
    WHEN 'withdraw'::text THEN 'withdrawn'::text
    ELSE 'recorded'::text
END"#,
    ),
    (
        "case_procedural_fact_revisions",
        "submission_view",
        r#"procedural_fact_submission(submission_canonical)"#,
    ),
];
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    key(client, TABLES[0], "p", &["family", "id"], None)?;
    key(client, TABLES[0], "u", &["family", "id", "case_id"], None)?;
    key(client, TABLES[1], "p", &["family", "id", "revision"], None)?;
    key(
        client,
        TABLES[1],
        "u",
        &["operation_id"],
        Some("procedural_fact_operation_unique"),
    )?;
    for (table, target, names, targets, deferred) in [
        (TABLES[0], "cases", vec!["case_id"], vec!["id"], false),
        (
            TABLES[0],
            TABLES[0],
            vec!["parent_family", "parent_resolution_id", "case_id"],
            vec!["family", "id", "case_id"],
            false,
        ),
        (
            TABLES[0],
            TABLES[1],
            vec!["family", "id", "initial_revision"],
            vec!["family", "id", "revision"],
            true,
        ),
        (
            TABLES[1],
            TABLES[0],
            vec!["family", "id", "case_id"],
            vec!["family", "id", "case_id"],
            false,
        ),
        (TABLES[1], "users", vec!["recorded_by"], vec!["id"], false),
        (
            TABLES[1],
            "case_administration_revisions",
            vec!["case_id", "recorded_administration_revision"],
            vec!["case_id", "revision"],
            false,
        ),
    ] {
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=$1::text::regclass
             AND c.confrelid=$2::text::regclass AND c.contype='f' AND c.convalidated
             AND c.condeferrable=$5 AND c.condeferred=$5 AND c.confupdtype='a' AND c.confdeltype='a'
             AND c.confmatchtype='s'
             AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[]
             AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$4::text[])",
            &[&table,&target,&names,&targets,&deferred],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, count) in [(TABLES[0], 8_i64), (TABLES[1], 19_i64)] {
        let actual: i64 = client.query_one(
            "SELECT count(*) FROM pg_constraint WHERE conrelid=$1::text::regclass AND contype<>'n'",
            &[&table],
        ).map_err(port)?.get(0);
        if actual != count {
            return Err(incomplete());
        }
    }
    expressions(client)
}
fn key<C: GenericClient>(
    client: &mut C,
    table: &str,
    kind: &str,
    columns: &[&str],
    name: Option<&str>,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=$1::text::regclass
         AND c.contype::text=$2 AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
         AND ($4::text IS NULL OR c.conname::text=$4)
         AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[])",
        &[&table,&kind,&columns,&name],
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
fn expressions<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let checks = client.query(
        "SELECT conrelid::regclass::text,conname::text,pg_get_expr(conbin,conrelid),convalidated,connoinherit
         FROM pg_constraint WHERE conrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND contype='c'",
        &[&&TABLES[..]],
    ).map_err(port)?;
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
    let expressions = client
        .query(
            "SELECT d.adrelid::regclass::text,a.attname::text,pg_get_expr(d.adbin,d.adrelid)
         FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum
         WHERE d.adrelid IN (SELECT t::regclass FROM unnest($1::text[]) t)",
            &[&&TABLES[..]],
        )
        .map_err(port)?;
    if expressions.len() != EXPRESSIONS.len()
        || EXPRESSIONS.iter().any(|(table, name, expression)| {
            !expressions.iter().any(|r| {
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
