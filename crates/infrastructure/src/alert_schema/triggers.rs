use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        for (name, function, kind) in [
            ("alert_lock", "lock_alert_mutations()", 22_i16),
            ("alert_protected", "preserve_alert_rows()", 42),
            ("alert_update", "validate_alert_update()", 19),
        ] {
            ordinary(client, table, name, function, kind)?;
        }
        let expected: i64 = if ["alert_email_attempts", "alert_email_outbox"].contains(&table) {
            4
        } else {
            3
        };
        let altered: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND NOT tgisinternal)<>$2",
                &[&table, &expected],
            )
            .map_err(port)?
            .get(0);
        if altered {
            return Err(incomplete());
        }
    }
    ordinary(
        client,
        "alert_email_attempts",
        "alert_attempt_insert",
        "validate_alert_attempt()",
        7,
    )?;
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint
        JOIN pg_class r ON r.oid=t.tgrelid
        WHERE t.tgrelid='alert_email_outbox'::regclass AND t.tgname='alert_outbox_ledger'
            AND t.tgfoid='validate_alert_outbox_ledger()'::regprocedure AND t.tgtype=21
            AND t.tgenabled IN ('O','A') AND NOT t.tgisinternal
            AND t.tgdeferrable AND t.tginitdeferred AND t.tgconstrrelid=0 AND t.tgconstrindid=0
            AND t.tgqual IS NULL AND t.tgnargs=0 AND octet_length(t.tgargs)=0
            AND t.tgattr=''::int2vector AND t.tgoldtable IS NULL AND t.tgnewtable IS NULL
            AND c.conrelid=t.tgrelid AND c.conname=t.tgname AND c.contype='t'
            AND c.connamespace=r.relnamespace AND c.convalidated AND c.condeferrable AND c.condeferred
            AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND c.confrelid=0 AND c.conindid=0 AND c.conbin IS NULL
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            AND (SELECT count(*) FROM pg_trigger linked WHERE linked.tgconstraint=c.oid)=1)", &[]
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
