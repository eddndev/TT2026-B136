use super::{constraints, functions, incomplete, indexes, port, CURSOR, JOBS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_class c
            JOIN pg_class d ON d.oid='case_deadlines'::regclass
            WHERE c.oid=to_regclass($1) AND c.relnamespace=d.relnamespace
                AND c.relkind='r' AND c.relpersistence='p' AND NOT c.relispartition
                AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity
                AND NOT EXISTS(SELECT 1 FROM pg_inherits
                    WHERE inhrelid=c.oid OR inhparent=c.oid)
                AND NOT EXISTS(SELECT 1 FROM pg_rewrite WHERE ev_class=c.oid))",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    columns(
        client,
        CURSOR,
        &[
            ("singleton", "boolean", true),
            ("completed_event_sequence", "bigint", false),
            ("active_event_sequence", "bigint", false),
            ("after_deadline_id", "uuid", false),
            ("bootstrap_after_deadline_id", "uuid", false),
        ],
    )?;
    columns(
        client,
        JOBS,
        &[
            ("id", "uuid", true),
            ("operation_id", "uuid", true),
            ("deadline_id", "uuid", true),
            ("case_id", "uuid", true),
            ("event_sequence", "bigint", false),
            ("bootstrap_policy_version", "smallint", false),
            ("created_at_seconds", "bigint", true),
            ("created_at_nanoseconds", "integer", true),
        ],
    )?;
    constraints::validate(client)?;
    indexes::validate(client)?;
    functions::validate(client)?;
    for (table, name, function, kind) in [
        (JOBS, "deadline_job_lock", "lock_deadline_dispatch()", 6_i16),
        (
            JOBS,
            "deadline_job_insert",
            "validate_deadline_reevaluation_job()",
            7,
        ),
        (
            JOBS,
            "deadline_job_immutable",
            "preserve_deadline_dispatch()",
            58,
        ),
        (
            CURSOR,
            "deadline_dispatch_lock",
            "lock_deadline_dispatch()",
            18,
        ),
        (
            CURSOR,
            "deadline_dispatch_update",
            "validate_deadline_dispatch_cursor()",
            19,
        ),
        (
            CURSOR,
            "deadline_dispatch_protected",
            "preserve_deadline_dispatch()",
            46,
        ),
        (
            "case_deadline_revisions",
            "deadline_operation_reserved",
            "reserve_deadline_job_operation()",
            7,
        ),
    ] {
        trigger(client, table, name, function, kind)?;
    }
    for table in TABLES {
        let altered: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND NOT tgisinternal)<>3",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if altered {
            return Err(incomplete());
        }
    }
    Ok(())
}

fn columns<C: GenericClient>(
    client: &mut C,
    table: &str,
    expected: &[(&str, &str, bool)],
) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
            a.attgenerated::text,a.attidentity::text,a.attcollation=0,
            EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
        FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
            AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
            &[&table],
        )
        .map_err(port)?;
    if rows.len() != expected.len()
        || rows
            .iter()
            .zip(expected)
            .any(|(row, (name, kind, required))| {
                row.get::<_, String>(0) != *name
                    || row.get::<_, String>(1) != *kind
                    || row.get::<_, bool>(2) != *required
                    || !row.get::<_, String>(3).is_empty()
                    || !row.get::<_, String>(4).is_empty()
                    || !row.get::<_, bool>(5)
                    || row.get::<_, bool>(6)
            })
    {
        return Err(incomplete());
    }
    Ok(())
}

fn trigger<C: GenericClient>(
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
            AND NOT tginitdeferred AND tgqual IS NULL AND tgnargs=0
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
