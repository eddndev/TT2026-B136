//! Exact keys, references and checks for durable deadline dispatch.
use super::{incomplete, port, CURSOR, JOBS};
use application::ApplicationError;
use postgres::GenericClient;

const CHECKS: &[(&str, &str, &str)] = &[
    (
        CURSOR,
        "deadline_dispatch_singleton",
        "(singleton IS TRUE)",
    ),
    (
        CURSOR,
        "deadline_dispatch_position",
        "((((completed_event_sequence IS NULL) OR (completed_event_sequence > 0))
            AND (num_nonnulls(active_event_sequence, after_deadline_id) = ANY (ARRAY[0, 2]))
            AND ((active_event_sequence IS NULL)
                OR (active_event_sequence > COALESCE(completed_event_sequence, (0)::bigint)))) IS TRUE)",
    ),
    (
        JOBS,
        "deadline_job_cause",
        "((((event_sequence IS NOT NULL) AND (event_sequence > 0)
            AND (bootstrap_policy_version IS NULL))
            OR ((event_sequence IS NULL) AND (bootstrap_policy_version = 1))) IS TRUE)",
    ),
    (
        JOBS,
        "deadline_job_seconds",
        "((created_at_seconds >= '-62135596800'::bigint)
            AND (created_at_seconds <= '253402300799'::bigint))",
    ),
    (
        JOBS,
        "deadline_job_nanoseconds",
        "((created_at_nanoseconds >= 0) AND (created_at_nanoseconds <= 999999999))",
    ),
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, name, kind, columns) in [
        (
            CURSOR,
            "deadline_dispatch_primary",
            "p",
            &["singleton"] as &[&str],
        ),
        (JOBS, "deadline_job_primary", "p", &["id"] as &[&str]),
        (
            JOBS,
            "deadline_job_operation",
            "u",
            &["operation_id"] as &[&str],
        ),
    ] {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
                JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
                JOIN pg_catalog.pg_class i ON i.oid=c.conindid
                WHERE c.conrelid=$1::text::regclass AND c.conname=$2
                    AND c.contype::text=$3 AND c.convalidated
                    AND NOT c.condeferrable AND NOT c.condeferred
                    AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                    AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                    AND i.relname=c.conname AND i.relnamespace=t.relnamespace
                    AND ARRAY(SELECT a.attname::text
                        FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                        JOIN pg_catalog.pg_attribute a
                            ON a.attrelid=c.conrelid AND a.attnum=k.num
                        ORDER BY k.n)=$4::text[])",
                &[&table, &name, &kind, &columns],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, name, target, columns, targets) in [
        (
            CURSOR,
            "deadline_dispatch_completed",
            "deadline_source_events",
            &["completed_event_sequence"] as &[&str],
            &["sequence"] as &[&str],
        ),
        (
            CURSOR,
            "deadline_dispatch_active",
            "deadline_source_events",
            &["active_event_sequence"] as &[&str],
            &["sequence"] as &[&str],
        ),
        (
            JOBS,
            "deadline_job_root",
            "case_deadlines",
            &["deadline_id", "case_id"] as &[&str],
            &["id", "case_id"] as &[&str],
        ),
        (
            JOBS,
            "deadline_job_event",
            "deadline_source_events",
            &["event_sequence"] as &[&str],
            &["sequence"] as &[&str],
        ),
    ] {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
                WHERE c.conrelid=$1::text::regclass AND c.conname=$2
                    AND c.confrelid=$3::text::regclass AND c.contype='f'
                    AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
                    AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                    AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                    AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
                    AND ARRAY(SELECT a.attname::text
                        FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                        JOIN pg_catalog.pg_attribute a
                            ON a.attrelid=c.conrelid AND a.attnum=k.num
                        ORDER BY k.n)=$4::text[]
                    AND ARRAY(SELECT a.attname::text
                        FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                        JOIN pg_catalog.pg_attribute a
                            ON a.attrelid=c.confrelid AND a.attnum=k.num
                        ORDER BY k.n)=$5::text[]
                    AND (SELECT count(*) FROM pg_catalog.pg_trigger t
                        WHERE t.tgconstraint=c.oid AND t.tgisinternal
                            AND t.tgenabled IN ('O','A'))=4
                    AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t
                        WHERE t.tgconstraint=c.oid AND t.tgenabled NOT IN ('O','A')))",
                &[&table, &name, &target, &columns, &targets],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, expected) in [(CURSOR, 5_i64), (JOBS, 7_i64)] {
        // PostgreSQL 18 also reports NOT NULL constraints in this catalog.
        let count: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_catalog.pg_constraint
                WHERE conrelid=$1::text::regclass AND contype<>'n'",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if count != expected {
            return Err(incomplete());
        }
    }
    checks(client)
}

fn checks<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT t.relname::text,c.conname::text,pg_catalog.pg_get_expr(c.conbin,c.conrelid),
            c.convalidated AND NOT c.connoinherit AND c.conislocal
                AND c.coninhcount=0 AND c.conparentid=0
                AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
        FROM pg_catalog.pg_constraint c JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
        WHERE c.conrelid IN ($1::text::regclass,$2::text::regclass) AND c.contype='c'",
            &[&CURSOR, &JOBS],
        )
        .map_err(port)?;
    if rows.len() != CHECKS.len()
        || CHECKS.iter().any(|(table, name, expression)| {
            !rows.iter().any(|row| {
                row.get::<_, String>(0) == *table
                    && row.get::<_, String>(1) == *name
                    && normalize(&row.get::<_, String>(2)) == normalize(expression)
                    && row.get::<_, bool>(3)
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
