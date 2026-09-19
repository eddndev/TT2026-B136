//! Exact index keys, ordering and operator semantics for worker ledgers.
use super::{incomplete, port, ATTEMPTS, RESULTS};
use application::ApplicationError;
use postgres::GenericClient;

struct Index {
    table: &'static str,
    name: &'static str,
    primary: bool,
    unique: bool,
    columns: &'static [&'static str],
    operators: &'static [&'static str],
    options: &'static [i16],
}
const INDEXES: &[Index] = &[
    Index {
        table: RESULTS,
        name: "deadline_worker_result_primary",
        primary: true,
        unique: true,
        columns: &["job_id"],
        operators: &["uuid_ops"],
        options: &[0],
    },
    Index {
        table: ATTEMPTS,
        name: "deadline_worker_attempt_primary",
        primary: true,
        unique: true,
        columns: &["attempt_id"],
        operators: &["uuid_ops"],
        options: &[0],
    },
    Index {
        table: ATTEMPTS,
        name: "deadline_worker_attempt_number",
        primary: false,
        unique: true,
        columns: &["job_id", "attempt_number"],
        operators: &["uuid_ops", "int8_ops"],
        options: &[0, 0],
    },
    Index {
        table: ATTEMPTS,
        name: "deadline_worker_attempt_latest",
        primary: false,
        unique: false,
        columns: &["job_id", "attempt_number"],
        operators: &["uuid_ops", "int8_ops"],
        options: &[0, 3],
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
            WHERE i.indrelid=$1::text::regclass AND c.relname=$2 AND c.relnamespace=t.relnamespace
                AND c.relkind='i' AND c.relpersistence='p' AND NOT c.relispartition
                AND am.amname='btree' AND i.indisprimary=$3 AND i.indisunique=$4
                AND i.indisvalid AND i.indisready AND i.indislive AND i.indimmediate
                AND NOT i.indisexclusion AND NOT i.indnullsnotdistinct
                AND i.indexprs IS NULL AND i.indpred IS NULL
                AND i.indnatts=cardinality($5::text[]) AND i.indnkeyatts=cardinality($5::text[])
                AND ARRAY(SELECT a.attname::text
                    FROM unnest(i.indkey::smallint[]) WITH ORDINALITY k(num,n)
                    JOIN pg_catalog.pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num
                    ORDER BY k.n)=$5::text[]
                AND ARRAY(SELECT op.opcname::text
                    FROM unnest(i.indclass::oid[]) WITH ORDINALITY k(id,n)
                    JOIN pg_catalog.pg_opclass op ON op.oid=k.id
                    JOIN pg_catalog.pg_namespace ns ON ns.oid=op.opcnamespace
                    WHERE ns.nspname='pg_catalog' AND op.opcmethod=c.relam AND op.opcdefault
                    ORDER BY k.n)=$6::text[]
                AND ARRAY(SELECT value FROM unnest(i.indoption::smallint[]) value)=$7::smallint[]
                AND NOT EXISTS(SELECT 1 FROM unnest(i.indcollation::oid[]) value WHERE value<>0)
                AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_inherits
                    WHERE inhrelid=i.indexrelid OR inhparent=i.indexrelid))",
                &[
                    &spec.table,
                    &spec.name,
                    &spec.primary,
                    &spec.unique,
                    &spec.columns,
                    &spec.operators,
                    &spec.options,
                ],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, expected) in [(RESULTS, 1_i64), (ATTEMPTS, 3_i64)] {
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
