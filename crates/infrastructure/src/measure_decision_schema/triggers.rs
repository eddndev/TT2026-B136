use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        ordinary(
            client,
            table,
            "measure_decision_lock",
            "lock_measure_decision_history()",
            6,
        )?;
        ordinary(
            client,
            table,
            "measure_decision_immutable",
            "preserve_measure_decision_history()",
            58,
        )?;
        complete(client, table)?;
        let expected: i64 = if matches!(table, "case_measure_decisions" | "case_measure_revisions")
        {
            4
        } else {
            3
        };
        let altered: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass AND NOT tgisinternal)<>$2",
            &[&table, &expected],
        ).map_err(port)?.get(0);
        if altered {
            return Err(incomplete());
        }
    }
    ordinary(
        client,
        "case_measure_decisions",
        "measure_decision_capture",
        "enforce_measure_decision_capture()",
        7,
    )?;
    ordinary(
        client,
        "case_measure_revisions",
        "measure_revision_source",
        "enforce_measure_revision_source()",
        7,
    )
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

fn complete<C: GenericClient>(client: &mut C, table: &str) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint
            JOIN pg_class r ON r.oid=t.tgrelid
            WHERE t.tgrelid=$1::text::regclass AND t.tgname='measure_decision_complete'
                AND t.tgfoid='enforce_measure_decision_complete()'::regprocedure AND t.tgtype=5
                AND t.tgenabled IN ('O','A') AND NOT t.tgisinternal
                AND t.tgdeferrable AND t.tginitdeferred AND t.tgconstrrelid=0 AND t.tgconstrindid=0
                AND t.tgqual IS NULL AND t.tgnargs=0 AND octet_length(t.tgargs)=0
                AND t.tgattr=''::int2vector AND t.tgoldtable IS NULL AND t.tgnewtable IS NULL
                AND c.conrelid=t.tgrelid AND c.conname=t.tgname AND c.contype='t'
                AND c.connamespace=r.relnamespace AND c.condeferrable AND c.condeferred
                AND c.convalidated AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connoinherit AND c.conindid=0 AND c.confrelid=0 AND c.conbin IS NULL
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND (SELECT count(*) FROM pg_trigger linked WHERE linked.tgconstraint=c.oid)=1)",
            &[&table],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
