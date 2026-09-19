//! Exact keys and references; CHECK predicates are validated separately.
use super::{checks, incomplete, port, ATTEMPTS, RESULTS};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, name, kind, columns) in [
        (
            RESULTS,
            "deadline_worker_result_primary",
            "p",
            &["job_id"] as &[&str],
        ),
        (
            ATTEMPTS,
            "deadline_worker_attempt_primary",
            "p",
            &["attempt_id"] as &[&str],
        ),
        (
            ATTEMPTS,
            "deadline_worker_attempt_number",
            "u",
            &["job_id", "attempt_number"] as &[&str],
        ),
    ] {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
            JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
            JOIN pg_catalog.pg_class i ON i.oid=c.conindid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype::text=$3
                AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND i.relname=c.conname AND i.relnamespace=t.relnamespace
                AND ARRAY(SELECT a.attname::text
                    FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                    JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num
                    ORDER BY k.n)=$4::text[])",
                &[&table, &name, &kind, &columns],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, name) in [
        (RESULTS, "deadline_worker_result_job"),
        (ATTEMPTS, "deadline_worker_attempt_job"),
    ] {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
            JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2
                AND c.confrelid='deadline_reevaluation_jobs'::regclass AND c.contype='f'
                AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
                AND ARRAY(SELECT a.attname::text
                    FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                    JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num
                    ORDER BY k.n)=ARRAY['job_id']::text[]
                AND ARRAY(SELECT a.attname::text
                    FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                    JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num
                    ORDER BY k.n)=ARRAY['id']::text[]
                AND (SELECT count(*) FROM pg_catalog.pg_trigger g
                    WHERE g.tgconstraint=c.oid AND g.tgisinternal AND g.tgenabled IN ('O','A'))=4
                AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger g
                    WHERE g.tgconstraint=c.oid AND g.tgenabled NOT IN ('O','A')))",
                &[&table, &name],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, expected) in [(RESULTS, 7_i64), (ATTEMPTS, 9_i64)] {
        // PostgreSQL 18 also exposes NOT NULL constraints through pg_constraint.
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
    checks::validate(client)
}
