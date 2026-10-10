use super::{incomplete, port, specifications::FOREIGN_KEYS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for &(table, name, _, target, _, deferred) in FOREIGN_KEYS {
        foreign_key(client, table, name, target, deferred)?;
    }
    for (name, function, kind) in [
        (
            "individual_signature_lock",
            "individual_signature_lock()",
            6,
        ),
        (
            "individual_signature_immutable",
            "individual_signature_preserve()",
            58,
        ),
        (
            "individual_signature_register",
            "individual_signature_register()",
            7,
        ),
    ] {
        ordinary(client, TABLES[0], name, function, kind)?;
    }
    deferred(client, TABLES[0], "individual_signature_receipt_pair", 5)?;
    deferred(
        client,
        "documents",
        "individual_signature_document_pair",
        21,
    )?;
    let invalid: bool = client
        .query_one(
            "SELECT (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass)<>14
            OR EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))",
            &[&TABLES[0]],
        )
        .map_err(port)?
        .get(0);
    if invalid {
        return Err(incomplete());
    }
    Ok(())
}

fn deferred<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    kind: i16,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger g JOIN pg_constraint c ON c.oid=g.tgconstraint
        JOIN pg_class t ON t.oid=g.tgrelid WHERE g.tgrelid=$1::text::regclass AND g.tgname=$2
            AND g.tgfoid='individual_signature_pair()'::regprocedure AND g.tgtype=$3
            AND g.tgenabled IN ('O','A') AND NOT g.tgisinternal AND g.tgdeferrable AND g.tginitdeferred
            AND g.tgconstrrelid=0 AND g.tgconstrindid=0 AND g.tgqual IS NULL AND g.tgnargs=0
            AND octet_length(g.tgargs)=0 AND g.tgattr=''::int2vector
            AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND c.conname=g.tgname AND c.contype='t' AND c.conrelid=t.oid
            AND c.connamespace=t.relnamespace AND c.condeferrable AND c.condeferred
            AND c.convalidated AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false')",
        &[&table, &name, &kind],
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

fn ordinary<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    function: &str,
    kind: i16,
) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
            AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4
            AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable
            AND NOT tginitdeferred AND tgconstraint=0 AND tgconstrrelid=0 AND tgconstrindid=0
            AND tgqual IS NULL AND tgnargs=0 AND octet_length(tgargs)=0
            AND tgattr=''::int2vector AND tgoldtable IS NULL AND tgnewtable IS NULL)",
            &[&table, &name, &function, &kind],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

fn foreign_key<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    target: &str,
    deferred: bool,
) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT g.tgrelid=c.conrelid,g.tgrelid=c.confrelid,g.tgtype,
            g.tgfoid::regprocedure::text,g.tgisinternal AND g.tgenabled IN ('O','A')
            AND g.tgdeferrable=$3 AND g.tginitdeferred=$3
            AND g.tgconstrindid=c.conindid AND g.tgqual IS NULL AND g.tgnargs=0
            AND octet_length(g.tgargs)=0 AND g.tgattr=''::int2vector
            AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND g.tgconstrrelid=CASE WHEN g.tgrelid=c.conrelid THEN c.confrelid ELSE c.conrelid END
        FROM pg_trigger g JOIN pg_constraint c ON c.oid=g.tgconstraint
        WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='f'
            AND c.confrelid=$4::text::regclass ORDER BY g.tgrelid=c.conrelid,g.tgtype",
            &[&table, &name, &deferred, &target],
        )
        .map_err(port)?;
    let expected = [
        (false, true, 9, "\"RI_FKey_noaction_del\"()"),
        (false, true, 17, "\"RI_FKey_noaction_upd\"()"),
        (true, false, 5, "\"RI_FKey_check_ins\"()"),
        (true, false, 17, "\"RI_FKey_check_upd\"()"),
    ];
    if rows.len() != expected.len()
        || rows
            .iter()
            .zip(expected)
            .any(|(row, (child, parent, kind, function))| {
                row.get::<_, bool>(0) != child
                    || row.get::<_, bool>(1) != parent
                    || row.get::<_, i16>(2) != kind
                    || row.get::<_, String>(3) != function
                    || !row.get::<_, bool>(4)
            })
    {
        return Err(incomplete());
    }
    Ok(())
}
