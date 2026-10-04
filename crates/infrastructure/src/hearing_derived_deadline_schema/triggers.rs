use super::{incomplete, port, specifications::FOREIGN_KEYS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for &(table, name, _, target, _, deferred) in FOREIGN_KEYS {
        foreign_key(client, table, name, target, deferred)?;
    }
    let table = TABLES[0];
    let expected = 3 + i64::try_from(FOREIGN_KEYS.len()).map_err(|_| incomplete())? * 2;
    for (name, function, kind) in [
        (
            "hearing_derived_deadline_lock",
            "lock_hearing_derived_deadline_history()",
            6,
        ),
        (
            "hearing_derived_deadline_immutable",
            "preserve_hearing_derived_deadline_history()",
            58,
        ),
        (
            "hearing_derived_deadline_origin",
            "enforce_hearing_derived_deadline_origin()",
            7,
        ),
    ] {
        ordinary(client, table, name, function, kind)?;
    }
    let altered: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
            AND tgenabled NOT IN ('O','A'))
        OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
            AND NOT tgisinternal)<>3
        OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass)<>$2",
            &[&table, &expected],
        )
        .map_err(port)?
        .get(0);
    if altered {
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
