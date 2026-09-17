use super::{incomplete, port, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

const CHECKS: &[(&str, &str)] = &[
    (
        "deadline_source_kind",
        r#"((source_kind COLLATE "C") = ANY (ARRAY['resolution'::text, 'notification'::text, 'hearing_result'::text, 'calendar'::text, 'profile'::text]))"#,
    ),
    (
        "deadline_source_revision",
        "((revision >= 1) AND (revision <= '4294967295'::bigint))",
    ),
    ("deadline_source_sequence_positive", "(sequence > 0)"),
    (
        "deadline_source_case_shape",
        "((source_kind = 'profile'::text) OR ((source_kind = 'calendar'::text) = (case_id IS NULL)))",
    ),
    (
        "deadline_source_hearing_shape",
        "((source_kind = 'hearing_result'::text) = (hearing_id IS NOT NULL))",
    ),
];
const GENERATED: &[(&str, &str)] = &[
    ("fact_family", "CASE WHEN (source_kind = ANY (ARRAY['resolution'::text, 'notification'::text])) THEN source_kind ELSE NULL::text END"),
    ("hearing_result_id", "CASE WHEN (source_kind = 'hearing_result'::text) THEN source_id ELSE NULL::uuid END"),
    ("calendar_id", "CASE WHEN (source_kind = 'calendar'::text) THEN source_id ELSE NULL::uuid END"),
    ("profile_id", "CASE WHEN (source_kind = 'profile'::text) THEN source_id ELSE NULL::uuid END"),
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    key(client, "deadline_source_event_primary", "p", &["sequence"])?;
    key(
        client,
        "deadline_source_revision_unique",
        "u",
        &["source_kind", "source_id", "revision"],
    )?;
    for (name, target, columns, targets) in [
        (
            "deadline_source_fact_revision",
            "case_procedural_fact_revisions",
            vec!["fact_family", "source_id", "revision"],
            vec!["family", "id", "revision"],
        ),
        (
            "deadline_source_fact_scope",
            "case_procedural_facts",
            vec!["fact_family", "source_id", "case_id"],
            vec!["family", "id", "case_id"],
        ),
        (
            "deadline_source_hearing_revision",
            "case_hearing_result_revisions",
            vec!["hearing_result_id", "revision"],
            vec!["result_id", "revision"],
        ),
        (
            "deadline_source_hearing_scope",
            "case_hearing_results",
            vec!["hearing_result_id", "case_id", "hearing_id"],
            vec!["id", "case_id", "hearing_id"],
        ),
        (
            "deadline_source_calendar_revision",
            "judicial_calendar_revisions",
            vec!["calendar_id", "revision"],
            vec!["calendar_id", "revision"],
        ),
        (
            "deadline_source_profile_revision",
            "deadline_profile_revisions",
            vec!["profile_id", "revision"],
            vec!["profile_id", "revision"],
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint c
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.confrelid=$3::text::regclass
            AND c.contype='f' AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
            AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[]
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$5::text[])",
            &[&TABLE,&name,&target,&columns,&targets]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let count: i64 = client.query_one("SELECT count(*) FROM pg_constraint WHERE conrelid='deadline_source_events'::regclass AND contype<>'n'", &[]).map_err(port)?.get(0);
    if count != 13 {
        return Err(incomplete());
    }
    expressions(client)
}

fn key<C: GenericClient>(
    client: &mut C,
    name: &str,
    kind: &str,
    columns: &[&str],
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint c
        JOIN pg_index i ON i.indexrelid=c.conindid WHERE c.conrelid=$1::text::regclass AND c.conname=$2
        AND c.contype::text=$3 AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
        AND i.indisvalid AND i.indisready AND i.indisunique AND i.indpred IS NULL AND i.indexprs IS NULL
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[])",
        &[&TABLE,&name,&kind,&columns]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

fn expressions<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT conname::text,pg_get_expr(conbin,conrelid),convalidated,connoinherit
        FROM pg_constraint WHERE conrelid='deadline_source_events'::regclass AND contype='c'",
            &[],
        )
        .map_err(port)?;
    if rows.len() != CHECKS.len()
        || CHECKS.iter().any(|(name, expression)| {
            !rows.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && normalize(&r.get::<_, String>(1)) == normalize(expression)
                    && r.get::<_, bool>(2)
                    && !r.get::<_, bool>(3)
            })
        })
    {
        return Err(incomplete());
    }
    let rows = client
        .query(
            "SELECT a.attname::text,pg_get_expr(d.adbin,d.adrelid)
        FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum
        WHERE d.adrelid='deadline_source_events'::regclass",
            &[],
        )
        .map_err(port)?;
    if rows.len() != GENERATED.len()
        || GENERATED.iter().any(|(name, expression)| {
            !rows.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && normalize(&r.get::<_, String>(1)) == normalize(expression)
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
