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
            ("case_id", "uuid", true, ""),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        TABLES[1],
        &[
            ("deadline_id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("title", "text", true, ""),
            ("profile_id", "uuid", true, ""),
            ("profile_revision", "bigint", true, ""),
            ("input_canonical", "bytea", true, ""),
            ("input_view", "jsonb", false, "s"),
            ("result_canonical", "bytea", true, ""),
            ("observed_administration_revision", "bigint", false, ""),
            ("observed_administration_canonical", "bytea", true, ""),
            ("observed_administration_digest", "bytea", true, ""),
            ("responsible_id", "uuid", true, ""),
            ("responsible_email", "text", true, ""),
            ("responsible_role", "text", true, ""),
            ("attention", "jsonb", true, ""),
            ("operation_id", "uuid", true, ""),
            ("action", "text", true, ""),
            ("status", "text", false, "s"),
            ("reason", "text", false, ""),
            ("review_canonical", "bytea", true, ""),
            ("capture_canonical", "bytea", true, ""),
            ("review_digest", "bytea", true, ""),
            ("capture_digest", "bytea", true, ""),
            ("submission_canonical", "bytea", true, ""),
            ("submission_digest", "bytea", true, ""),
            ("submission_view", "jsonb", false, "s"),
            ("recorded_at_seconds", "bigint", true, ""),
            ("recorded_at_nanoseconds", "integer", true, ""),
            ("recorded_by", "uuid", false, ""),
            ("recorded_by_email", "text", false, ""),
            ("due_at_seconds", "bigint", false, ""),
            ("due_at_nanoseconds", "integer", false, ""),
            ("source_kind", "text", false, ""),
            ("source_id", "uuid", false, ""),
            ("source_revision", "bigint", false, ""),
            ("source_head_revision", "bigint", false, ""),
            ("source_hearing_id", "uuid", false, ""),
            ("source_parent_resolution_id", "uuid", false, ""),
            ("source_parent_resolution_revision", "bigint", false, ""),
            (
                "source_head_parent_resolution_revision",
                "bigint",
                false,
                "",
            ),
            ("calendar_id", "uuid", false, ""),
            ("calendar_revision", "bigint", false, ""),
            ("calendar_head_revision", "bigint", false, ""),
            ("source_fact_family", "text", false, "s"),
            ("source_fact_id", "uuid", false, "s"),
            ("source_result_id", "uuid", false, "s"),
            ("source_parent_family", "text", false, "s"),
            ("tracking_canonical", "bytea", false, ""),
            ("observations_canonical", "bytea", false, ""),
            ("tracking_administration_revision", "bigint", false, "s"),
            ("cause_event_sequence", "bigint", false, "s"),
        ],
    )?;
    constraints::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("deadline constraints: {e}"))
    })?;
    expressions::validate(client).map_err(|e| {
        ApplicationError::InvalidConfiguration(format!("deadline expressions: {e}"))
    })?;
    functions::validate(client)
        .map_err(|e| ApplicationError::InvalidConfiguration(format!("deadline functions: {e}")))?;
    for (table, trigger, function, kind) in [
        (
            TABLES[0],
            "deadline_immutable",
            "preserve_deadline_history()",
            58_i16,
        ),
        (
            TABLES[1],
            "deadline_immutable",
            "preserve_deadline_history()",
            58,
        ),
        (
            TABLES[1],
            "deadline_sequence",
            "enforce_deadline_sequence()",
            7,
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4 AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred AND tgqual IS NULL AND tgnargs=0 AND tgattr=''::int2vector)",&[&table,&trigger,&function,&kind]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let altered:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND tgenabled NOT IN ('O','A')) OR (SELECT count(*) FROM pg_trigger WHERE tgrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND NOT tgisinternal)<>5",&[&&TABLES[..]]).map_err(port)?.get(0);
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
