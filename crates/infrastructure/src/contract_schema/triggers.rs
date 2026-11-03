use super::{incomplete, port, specifications::PAIRS, APPEND_ONLY, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

/// pg_trigger.tgtype of a statement trigger before INSERT.
const BEFORE_INSERT_STATEMENT: i16 = 6;
/// Before UPDATE, DELETE or TRUNCATE, for each statement.
const BEFORE_CHANGE_STATEMENT: i16 = 58;
/// Before DELETE or TRUNCATE, for each statement.
const BEFORE_REMOVAL_STATEMENT: i16 = 42;
const BEFORE_INSERT_ROW: i16 = 7;
const BEFORE_UPDATE_ROW: i16 = 19;
const AFTER_INSERT_ROW: i16 = 5;

const REGISTERS: [(&str, &str, &str, i16); 6] = [
    (
        "contract_events",
        "contract_event_register",
        "contract_event_register()",
        BEFORE_INSERT_ROW,
    ),
    (
        "contract_signature_requests",
        "contract_request_register",
        "contract_request_register()",
        BEFORE_INSERT_ROW,
    ),
    (
        "contract_party_signatures",
        "contract_signature_register",
        "contract_signature_register()",
        BEFORE_INSERT_ROW,
    ),
    (
        "contract_completions",
        "contract_completion_register",
        "contract_completion_register()",
        BEFORE_INSERT_ROW,
    ),
    (
        "contract_notifications",
        "contract_notification_register",
        "contract_notification_register()",
        BEFORE_INSERT_ROW,
    ),
    (
        "contract_notifications",
        "contract_notification_guard",
        "contract_notification_guard()",
        BEFORE_UPDATE_ROW,
    ),
];

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        ordinary(
            client,
            table,
            "contract_lock",
            "contract_lock()",
            BEFORE_INSERT_STATEMENT,
        )?;
        let immutable = if APPEND_ONLY.contains(&table) {
            BEFORE_CHANGE_STATEMENT
        } else {
            BEFORE_REMOVAL_STATEMENT
        };
        ordinary(
            client,
            table,
            "contract_immutable",
            "contract_preserve()",
            immutable,
        )?;
        let expected = 2
            + REGISTERS.iter().filter(|entry| entry.0 == table).count()
            + PAIRS.iter().filter(|pair| pair.0 == table).count();
        let altered: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND NOT tgisinternal)<>$2",
                &[&table, &i64::try_from(expected).map_err(|_| incomplete())?],
            )
            .map_err(port)?
            .get(0);
        if altered {
            return Err(incomplete());
        }
    }
    for (table, name, function, kind) in REGISTERS {
        ordinary(client, table, name, function, kind)?;
    }
    for (table, name) in PAIRS {
        deferred(client, table, name)?;
    }
    version_guard(client)
}

fn ordinary<C: CatalogClient>(
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

fn deferred<C: CatalogClient>(
    client: &mut C,
    table: &str,
    name: &str,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger g JOIN pg_constraint c ON c.oid=g.tgconstraint
        JOIN pg_class t ON t.oid=g.tgrelid WHERE g.tgrelid=$1::text::regclass AND g.tgname=$2
            AND g.tgfoid='contract_completion_pair()'::regprocedure AND g.tgtype=$3
            AND g.tgenabled IN ('O','A') AND NOT g.tgisinternal AND g.tgdeferrable AND g.tginitdeferred
            AND g.tgconstrrelid=0 AND g.tgconstrindid=0 AND g.tgqual IS NULL AND g.tgnargs=0
            AND octet_length(g.tgargs)=0 AND g.tgattr=''::int2vector
            AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND c.conname=g.tgname AND c.contype='t' AND c.conrelid=t.oid
            AND c.connamespace=t.relnamespace AND c.condeferrable AND c.condeferred
            AND c.convalidated AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false')",
        &[&table, &name, &AFTER_INSERT_ROW],
    ).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

/// The guard fires only when `documents.evidence` is assigned.
fn version_guard<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger g JOIN pg_attribute a
                ON a.attrelid=g.tgrelid AND a.attname='evidence'
            WHERE g.tgrelid='documents'::regclass AND g.tgname='contract_version_guard'
            AND g.tgfoid='contract_version_guard()'::regprocedure AND g.tgtype=$1
            AND g.tgenabled IN ('O','A') AND NOT g.tgisinternal AND NOT g.tgdeferrable
            AND NOT g.tginitdeferred AND g.tgconstraint=0 AND g.tgconstrrelid=0
            AND g.tgconstrindid=0 AND g.tgqual IS NULL AND g.tgnargs=0
            AND octet_length(g.tgargs)=0 AND g.tgattr::text=a.attnum::text
            AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL)
            AND (SELECT count(*) FROM pg_trigger WHERE tgrelid='documents'::regclass
                AND tgfoid='contract_version_guard()'::regprocedure)=1",
            &[&BEFORE_UPDATE_ROW],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
