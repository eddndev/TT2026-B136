//! Exact index keys, ordering, collation and operator semantics.
use super::{incomplete, port, specifications::KEYS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

const LOOKUPS: &[(&str, &str, &[&str])] = &[(
    "case_resource_activity_associations",
    "resource_activity_case_resource_order",
    &["case_id", "resource_id", "id"],
)];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for &(table, name, columns, primary) in KEYS {
        validate_one(client, table, name, columns, primary, true)?;
    }
    for &(table, name, columns) in LOOKUPS {
        validate_one(client, table, name, columns, false, false)?;
    }
    for table in TABLES {
        let expected = KEYS.iter().filter(|key| key.0 == table).count()
            + LOOKUPS.iter().filter(|key| key.0 == table).count();
        let count: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_index WHERE indrelid=$1::text::regclass",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if usize::try_from(count).ok() != Some(expected) {
            return Err(incomplete());
        }
    }
    Ok(())
}

fn validate_one<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    columns: &[&str],
    primary: bool,
    unique: bool,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_index i
        JOIN pg_class t ON t.oid=i.indrelid JOIN pg_class c ON c.oid=i.indexrelid
        JOIN pg_am am ON am.oid=c.relam
        WHERE i.indrelid=$1::text::regclass AND c.relname=$2 AND c.relnamespace=t.relnamespace
            AND c.relowner=t.relowner AND c.relkind='i' AND c.relpersistence='p'
            AND NOT c.relispartition AND am.amname='btree'
            AND i.indisprimary=$3 AND i.indisunique=$4
            AND i.indisvalid AND i.indisready AND i.indislive AND i.indimmediate
            AND NOT i.indisexclusion AND NOT i.indnullsnotdistinct
            AND i.indexprs IS NULL AND i.indpred IS NULL
            AND i.indnatts=cardinality($5::text[]) AND i.indnkeyatts=cardinality($5::text[])
            AND ARRAY(SELECT a.attname::text FROM unnest(i.indkey::smallint[]) WITH ORDINALITY k(num,n)
                JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num
                ORDER BY k.n)=$5::text[]
            AND NOT EXISTS(SELECT 1 FROM unnest(i.indkey::smallint[],i.indclass::oid[],
                    i.indcollation::oid[],i.indoption::smallint[]) k(num,op,coll,options)
                JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num
                LEFT JOIN pg_opclass op ON op.oid=k.op
                WHERE op.opcnamespace<>'pg_catalog'::regnamespace OR op.opcmethod<>c.relam
                    OR NOT op.opcdefault OR op.opcintype<>a.atttypid
                    OR k.coll<>a.attcollation OR k.options<>0)
            AND NOT EXISTS(SELECT 1 FROM pg_inherits
                WHERE inhrelid=i.indexrelid OR inhparent=i.indexrelid))",
        &[&table, &name, &primary, &unique, &columns]
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
