//! Index keys, predicates and comparison semantics for durable dispatch.
use super::{incomplete, port, CURSOR, JOBS};
use application::ApplicationError;
use postgres::GenericClient;

struct Index {
    table: &'static str,
    name: &'static str,
    primary: bool,
    unique: bool,
    columns: &'static [&'static str],
    operator_classes: &'static [&'static str],
    predicate: Option<&'static str>,
}

const INDEXES: &[Index] = &[
    Index {
        table: CURSOR,
        name: "deadline_dispatch_primary",
        primary: true,
        unique: true,
        columns: &["singleton"],
        operator_classes: &["bool_ops"],
        predicate: None,
    },
    Index {
        table: JOBS,
        name: "deadline_job_primary",
        primary: true,
        unique: true,
        columns: &["id"],
        operator_classes: &["uuid_ops"],
        predicate: None,
    },
    Index {
        table: JOBS,
        name: "deadline_job_operation",
        primary: false,
        unique: true,
        columns: &["operation_id"],
        operator_classes: &["uuid_ops"],
        predicate: None,
    },
    Index {
        table: JOBS,
        name: "deadline_job_event_unique",
        primary: false,
        unique: true,
        columns: &["event_sequence", "deadline_id"],
        operator_classes: &["int8_ops", "uuid_ops"],
        predicate: Some("(event_sequence IS NOT NULL)"),
    },
    Index {
        table: JOBS,
        name: "deadline_job_bootstrap_unique",
        primary: false,
        unique: true,
        columns: &["deadline_id", "bootstrap_policy_version"],
        operator_classes: &["uuid_ops", "int2_ops"],
        predicate: Some("(event_sequence IS NULL)"),
    },
    Index {
        table: JOBS,
        name: "deadline_job_created_order",
        primary: false,
        unique: false,
        columns: &["created_at_seconds", "created_at_nanoseconds", "id"],
        operator_classes: &["int8_ops", "int4_ops", "uuid_ops"],
        predicate: None,
    },
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for spec in INDEXES {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_index i
                JOIN pg_catalog.pg_class t ON t.oid=i.indrelid
                JOIN pg_catalog.pg_class c ON c.oid=i.indexrelid
                JOIN pg_catalog.pg_am am ON am.oid=c.relam
                WHERE i.indrelid=$1::text::regclass AND c.relname=$2
                    AND c.relnamespace=t.relnamespace AND c.relkind='i'
                    AND c.relpersistence='p' AND NOT c.relispartition
                    AND am.amname='btree' AND i.indisprimary=$3 AND i.indisunique=$4
                    AND i.indisvalid AND i.indisready AND i.indislive AND i.indimmediate
                    AND NOT i.indisexclusion AND NOT i.indnullsnotdistinct
                    AND i.indexprs IS NULL
                    AND i.indnatts=cardinality($5::text[])
                    AND i.indnkeyatts=cardinality($5::text[])
                    AND ARRAY(SELECT a.attname::text
                        FROM unnest(i.indkey::smallint[]) WITH ORDINALITY k(num,n)
                        JOIN pg_catalog.pg_attribute a
                            ON a.attrelid=i.indrelid AND a.attnum=k.num
                        ORDER BY k.n)=$5::text[]
                    AND ARRAY(SELECT op.opcname::text
                        FROM unnest(i.indclass::oid[]) WITH ORDINALITY k(id,n)
                        JOIN pg_catalog.pg_opclass op ON op.oid=k.id
                        JOIN pg_catalog.pg_namespace ns ON ns.oid=op.opcnamespace
                        WHERE ns.nspname='pg_catalog' AND op.opcmethod=c.relam AND op.opcdefault
                        ORDER BY k.n)=$6::text[]
                    AND NOT EXISTS(SELECT 1 FROM unnest(i.indoption::smallint[]) AS o(value)
                        WHERE o.value<>0)
                    AND NOT EXISTS(SELECT 1 FROM unnest(i.indcollation::oid[]) AS o(value)
                        WHERE o.value<>0)
                    AND pg_catalog.pg_get_expr(i.indpred,i.indrelid) IS NOT DISTINCT FROM $7::text
                    AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_inherits
                        WHERE inhrelid=i.indexrelid OR inhparent=i.indexrelid))",
                &[
                    &spec.table,
                    &spec.name,
                    &spec.primary,
                    &spec.unique,
                    &spec.columns,
                    &spec.operator_classes,
                    &spec.predicate,
                ],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, expected) in [(CURSOR, 1_i64), (JOBS, 5_i64)] {
        let count: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_catalog.pg_index WHERE indrelid=$1::text::regclass",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if count != expected {
            return Err(incomplete());
        }
    }
    Ok(())
}
