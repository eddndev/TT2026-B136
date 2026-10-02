use super::{constraints::FOREIGN, incomplete, port, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger g JOIN pg_constraint c ON c.oid=g.tgconstraint
        JOIN pg_class t ON t.oid=g.tgrelid WHERE g.tgrelid=$1::text::regclass
            AND g.tgname='password_reset_requires_audit'
            AND g.tgfoid='password_reset_audit_receipt()'::regprocedure AND g.tgtype=17
            AND g.tgenabled IN ('O','A') AND NOT g.tgisinternal AND g.tgdeferrable AND g.tginitdeferred
            AND g.tgconstrrelid=0 AND g.tgconstrindid=0 AND g.tgqual IS NULL AND g.tgnargs=0
            AND octet_length(g.tgargs)=0 AND g.tgattr=''::int2vector
            AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND c.conname=g.tgname AND c.contype='t' AND c.conrelid=t.oid
            AND c.connamespace=t.relnamespace AND c.condeferrable AND c.condeferred
            AND c.convalidated AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0)", &[&TABLE],
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    for &(name, _, target, _, deferred) in FOREIGN {
        foreign_key(client, name, target, deferred)?;
    }
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass",
            &[&TABLE],
        )
        .map_err(port)?
        .get(0);
    if count != 5 {
        return Err(incomplete());
    }
    validate_audit_triggers(client)
}

fn foreign_key<C: GenericClient>(
    client: &mut C,
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
            &[&TABLE, &name, &deferred, &target],
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

pub(crate) fn validate_audit_triggers<C: GenericClient>(
    client: &mut C,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT (SELECT count(*) FROM pg_trigger WHERE tgrelid='audit_events'::regclass)=2
        AND NOT EXISTS(SELECT 1 FROM pg_trigger g LEFT JOIN pg_constraint c ON c.oid=g.tgconstraint
        WHERE g.tgrelid='audit_events'::regclass AND NOT COALESCE(
            g.tgisinternal AND g.tgenabled IN ('O','A') AND g.tgdeferrable AND g.tginitdeferred
            AND c.conrelid=to_regclass('password_reset_capabilities')
            AND c.conname='password_reset_capabilities_audit_sequence_fkey' AND c.contype='f'
            AND c.confrelid=g.tgrelid AND c.convalidated AND c.condeferrable AND c.condeferred
            AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
            AND g.tgconstrrelid=c.conrelid AND g.tgconstrindid=c.conindid
            AND g.tgqual IS NULL AND g.tgnargs=0 AND octet_length(g.tgargs)=0
            AND g.tgattr=''::int2vector AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND ((g.tgtype=9 AND g.tgfoid='pg_catalog.\"RI_FKey_noaction_del\"()'::regprocedure)
                OR (g.tgtype=17 AND g.tgfoid='pg_catalog.\"RI_FKey_noaction_upd\"()'::regprocedure)),false))",
        &[],
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
