use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, key) in [
        (TABLES[0], vec!["id"]),
        (TABLES[1], vec!["hearing_id", "revision"]),
    ] {
        key_constraint(client, table, "p", &key, false)?;
    }
    key_constraint(client, TABLES[0], "u", &["id", "case_id"], false)?;
    key_constraint(client, TABLES[1], "u", &["operation_id"], false)?;
    for (table, names) in [
        (TABLES[0], vec!["hearing_initial_revision"]),
        (
            TABLES[1],
            vec![
                "hearing_revision_range",
                "hearing_values_size",
                "hearing_values_hash",
                "hearing_receipt_projection",
                "hearing_receipt_context",
                "hearing_context_order",
                "hearing_kind_context",
                "hearing_action",
                "hearing_submission_size",
                "hearing_submission_hash",
                "hearing_scheduling_digest_size",
                "hearing_stage_revision_range",
                "hearing_stage_kind",
                "hearing_stage_digest_size",
                "hearing_recorded_digest_size",
                "hearing_recorded_time_range",
                "hearing_recorded_nanoseconds_range",
                "hearing_reason",
                "hearing_actor_email",
                "hearing_support_capture",
            ],
        ),
    ] {
        let valid: bool = client.query_one("SELECT
            (SELECT count(*) FROM pg_catalog.pg_constraint WHERE conrelid=$1::text::regclass
                AND contype='c' AND convalidated)=cardinality($2::text[])
            AND (SELECT count(*) FROM pg_catalog.pg_constraint WHERE conrelid=$1::text::regclass
                AND contype='c' AND convalidated AND conname::text=ANY($2::text[]))=cardinality($2::text[])",
            &[&table,&names]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, target, names, targets, deferred) in [
        (TABLES[0], "cases", vec!["case_id"], vec!["id"], false),
        (
            TABLES[0],
            TABLES[1],
            vec!["id", "initial_revision"],
            vec!["hearing_id", "revision"],
            true,
        ),
        (
            TABLES[1],
            TABLES[0],
            vec!["hearing_id", "case_id"],
            vec!["id", "case_id"],
            false,
        ),
        (TABLES[1], "users", vec!["recorded_by"], vec!["id"], false),
        (
            TABLES[1],
            "case_administration_revisions",
            vec!["case_id", "scheduling_administration_revision"],
            vec!["case_id", "revision"],
            false,
        ),
        (
            TABLES[1],
            "case_administration_revisions",
            vec!["case_id", "recorded_administration_revision"],
            vec!["case_id", "revision"],
            false,
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
            WHERE c.conrelid=$1::text::regclass AND c.confrelid=$2::text::regclass AND c.contype='f'
            AND c.convalidated AND c.condeferrable=$5 AND c.condeferred=$5 AND c.confupdtype='a' AND c.confdeltype='a'
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[]
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$4::text[])",
            &[&table,&target,&names,&targets,&deferred]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    Ok(())
}
fn key_constraint<C: GenericClient>(
    client: &mut C,
    table: &str,
    kind: &str,
    columns: &[&str],
    deferred: bool,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
        WHERE c.conrelid=$1::text::regclass AND c.contype::text=$2 AND c.convalidated
        AND c.condeferrable=$4 AND c.condeferred=$4
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[])",
        &[&table,&kind,&columns,&deferred]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
