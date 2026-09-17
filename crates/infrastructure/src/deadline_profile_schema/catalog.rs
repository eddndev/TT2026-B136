use super::{constraints, expressions, functions, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass($1) AND relkind='r' AND relpersistence='p' AND NOT relispartition AND NOT relrowsecurity AND NOT relforcerowsecurity AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=to_regclass($1) OR inhparent=to_regclass($1)))",&[&table]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    columns(
        client,
        TABLES[0],
        &[
            ("id", "uuid", true, ""),
            ("case_id", "uuid", false, ""),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        TABLES[1],
        &[
            ("profile_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("definition_canonical", "bytea", true, ""),
            ("definition_digest", "bytea", true, ""),
            ("definition_view", "jsonb", false, "s"),
            ("algorithm", "smallint", true, ""),
            ("operation_id", "uuid", true, ""),
            ("action", "text", true, ""),
            ("status", "text", false, "s"),
            ("reason", "text", false, ""),
            ("submission_canonical", "bytea", true, ""),
            ("submission_digest", "bytea", true, ""),
            ("submission_view", "jsonb", false, "s"),
            ("recorded_at_seconds", "bigint", true, ""),
            ("recorded_at_nanoseconds", "integer", true, ""),
            ("recorded_by", "uuid", true, ""),
            ("recorded_by_email", "text", true, ""),
        ],
    )?;
    constraints::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("deadline profile constraints: {e}"))
    })?;
    expressions::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("deadline profile expressions: {e}"))
    })?;
    functions::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("deadline profile functions: {e}"))
    })?;
    for (table, trigger, function, kind) in [
        (
            TABLES[0],
            "deadline_profile_immutable",
            "preserve_deadline_profile_history()",
            58_i16,
        ),
        (
            TABLES[1],
            "deadline_profile_immutable",
            "preserve_deadline_profile_history()",
            58,
        ),
        (
            TABLES[1],
            "deadline_profile_sequence",
            "enforce_deadline_profile_sequence()",
            7,
        ),
        (
            TABLES[1],
            "deadline_source_emit",
            "emit_deadline_source_event()",
            5,
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4 AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred AND tgqual IS NULL AND tgnargs=0 AND tgattr=''::int2vector)",&[&table,&trigger,&function,&kind]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let altered:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND tgenabled NOT IN ('O','A')) OR (SELECT count(*) FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND NOT tgisinternal)<>4",&[&&TABLES[..]]).map_err(port)?.get(0);
    if altered {
        return Err(incomplete());
    }
    Ok(())
}
fn columns<C: GenericClient>(
    client: &mut C,
    table: &str,
    expected: &[(&str, &str, bool, &str)],
) -> Result<(), ApplicationError> {
    let rows=client.query("SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,attgenerated::text,attidentity::text FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped",&[&table]).map_err(port)?;
    if rows.len() != expected.len()
        || expected.iter().any(|(name, kind, required, generated)| {
            !rows.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && r.get::<_, String>(1) == *kind
                    && r.get::<_, bool>(2) == *required
                    && r.get::<_, String>(3) == *generated
                    && r.get::<_, String>(4).is_empty()
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}
