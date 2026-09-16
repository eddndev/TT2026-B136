use super::{constraints, expressions, functions, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass($1) AND relkind='r' AND NOT relrowsecurity AND NOT relforcerowsecurity)",&[&table]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    columns(
        client,
        TABLES[0],
        &[
            ("id", "uuid", true, ""),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        TABLES[1],
        &[
            ("calendar_id", "uuid", true, ""),
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
            ("recorded_at_seconds", "bigint", true, ""),
            ("recorded_at_nanoseconds", "integer", true, ""),
            ("recorded_by", "uuid", true, ""),
            ("recorded_by_email", "text", true, ""),
        ],
    )?;
    constraints::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("calendar constraints: {e}"))
    })?;
    expressions::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("calendar expressions: {e}"))
    })?;
    functions::validate(client)
        .map_err(|e| ApplicationError::InvalidConfiguration(format!("calendar functions: {e}")))?;
    for (table, trigger, function, kind) in [
        (
            TABLES[0],
            "judicial_calendar_immutable",
            "preserve_judicial_calendar_history()",
            58_i16,
        ),
        (
            TABLES[1],
            "judicial_calendar_immutable",
            "preserve_judicial_calendar_history()",
            58,
        ),
        (
            TABLES[1],
            "judicial_calendar_sequence",
            "enforce_judicial_calendar_sequence()",
            7,
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4 AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred AND tgqual IS NULL AND tgnargs=0 AND tgattr=''::int2vector)",&[&table,&trigger,&function,&kind]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let altered:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND tgenabled NOT IN ('O','A')) OR (SELECT count(*) FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND NOT tgisinternal)<>3",&[&&TABLES[..]]).map_err(port)?.get(0);
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
