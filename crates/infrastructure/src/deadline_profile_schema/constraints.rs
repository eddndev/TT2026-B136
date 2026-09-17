use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    key_constraint(client, TABLES[0], "p", &["id"], false)?;
    key_constraint(client, TABLES[1], "p", &["profile_id", "revision"], false)?;
    key_constraint(client, TABLES[1], "u", &["operation_id"], false)?;
    for (table, target, names, targets, deferred) in [
        (
            TABLES[0],
            TABLES[1],
            vec!["id", "initial_revision"],
            vec!["profile_id", "revision"],
            true,
        ),
        (TABLES[1], TABLES[0], vec!["profile_id"], vec!["id"], false),
        (TABLES[1], "users", vec!["recorded_by"], vec!["id"], false),
        (TABLES[0], "cases", vec!["case_id"], vec!["id"], false),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=$1::text::regclass AND c.confrelid=$2::text::regclass AND c.contype='f' AND c.convalidated AND c.condeferrable=$5 AND c.condeferred=$5 AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[]
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n) JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$4::text[])",&[&table,&target,&names,&targets,&deferred]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, count) in [(TABLES[0], 4_i64), (TABLES[1], 16_i64)] {
        let actual: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_constraint WHERE conrelid=$1::text::regclass AND contype<>'n'",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if actual != count {
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
