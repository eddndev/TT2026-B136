use super::{constraints, incomplete, port, HELPERS, TABLES, TRIGGERS};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        if !client
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .map_err(port)?
            .get::<_, bool>(0)
        {
            return Err(incomplete());
        }
    }
    columns(
        client,
        "case_hearings",
        &[
            ("id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        "case_hearing_revisions",
        &[
            ("hearing_id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("values_canonical", "bytea", true, ""),
            ("values_digest", "bytea", true, ""),
            ("values_view", "jsonb", false, "s"),
            ("operation_id", "uuid", true, ""),
            ("action", "text", true, ""),
            ("status", "text", false, "s"),
            ("reason", "text", false, ""),
            ("submission_canonical", "bytea", true, ""),
            ("submission_digest", "bytea", true, ""),
            ("submission_view", "jsonb", false, "s"),
            ("scheduling_administration_revision", "bigint", true, ""),
            ("scheduling_administration_digest", "bytea", true, ""),
            ("scheduling_stage_revision", "bigint", true, ""),
            ("scheduling_stage", "text", true, ""),
            ("scheduling_stage_digest", "bytea", false, ""),
            ("recorded_administration_revision", "bigint", true, ""),
            ("recorded_administration_digest", "bytea", true, ""),
            ("recorded_at_seconds", "bigint", true, ""),
            ("recorded_at_nanoseconds", "integer", true, ""),
            ("recorded_by", "uuid", true, ""),
            ("recorded_by_email", "text", true, ""),
            ("support_name", "text", false, ""),
            ("support_format", "text", false, ""),
            ("support_policy", "text", false, ""),
        ],
    )?;
    constraints::validate(client)?;
    for function in HELPERS {
        function_kind(client, function, "i", "jsonb")?;
    }
    for function in TRIGGERS {
        function_kind(client, function, "v", "trigger")?;
    }
    for (column, function) in [
        ("values_view", "hearing_values(bytea)"),
        ("submission_view", "hearing_submission(bytea)"),
    ] {
        let generated: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_attrdef d
            JOIN pg_catalog.pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum
            JOIN pg_catalog.pg_depend b ON b.classid='pg_attrdef'::regclass AND b.objid=d.oid
            WHERE a.attrelid='case_hearing_revisions'::regclass AND a.attname=$1
                AND b.refclassid='pg_proc'::regclass AND b.refobjid=$2::text::regprocedure)",
                &[&column, &function],
            )
            .map_err(port)?
            .get(0);
        if !generated {
            return Err(incomplete());
        }
    }
    for table in TABLES {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t
            WHERE tgrelid=$1::text::regclass AND tgname='hearing_immutable' AND tgtype=58
                AND tgfoid='preserve_hearing_history()'::regprocedure AND tgenabled IN ('O','A')
                AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred)",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let sequence: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger
        WHERE tgrelid='case_hearing_revisions'::regclass AND tgname='hearing_sequence' AND tgtype=7
            AND tgfoid='enforce_hearing_sequence()'::regprocedure AND tgenabled IN ('O','A')
            AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred)",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !sequence {
        return Err(incomplete());
    }
    let disabled: bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t
        JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint
        WHERE c.conrelid IN (SELECT n::regclass FROM unnest($1::text[]) n) AND t.tgenabled NOT IN ('O','A'))",
        &[&&TABLES[..]]).map_err(port)?.get(0);
    if disabled {
        return Err(incomplete());
    }
    Ok(())
}
fn columns<C: GenericClient>(
    client: &mut C,
    table: &str,
    expected: &[(&str, &str, bool, &str)],
) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,attgenerated::text
        FROM pg_catalog.pg_attribute a JOIN pg_catalog.pg_class c ON c.oid=a.attrelid
        WHERE a.attrelid=$1::text::regclass AND c.relkind='r' AND attnum>0 AND NOT attisdropped",
            &[&table],
        )
        .map_err(port)?;
    if rows.len() != expected.len()
        || expected.iter().any(|(name, kind, required, generated)| {
            !rows.iter().any(|row| {
                row.get::<_, String>(0) == *name
                    && row.get::<_, String>(1) == *kind
                    && row.get::<_, bool>(2) == *required
                    && row.get::<_, String>(3) == *generated
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}
fn function_kind<C: GenericClient>(
    client: &mut C,
    name: &str,
    volatility: &str,
    returns: &str,
) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_proc p
        WHERE p.oid=to_regprocedure($1) AND p.provolatile::text=$2
            AND p.prorettype=$3::text::regtype AND NOT p.prosecdef)",
            &[&name, &volatility, &returns],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
