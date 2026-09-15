use application::ApplicationError;
use postgres::Client;

const TABLE: &str = "case_stage_revisions";

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let exists: bool = client
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&TABLE])
        .map_err(port)?
        .get(0);
    if !exists {
        return Err(incomplete());
    }
    for (signature, volatility) in [
        (
            "case_stage_time_bounds(text,date,bigint,integer,integer)",
            "i",
        ),
        (
            "case_stage_time_bytes(text,date,bigint,integer,integer)",
            "i",
        ),
        (
            "case_stage_support_valid(uuid,bigint,bytea,text,text,text)",
            "i",
        ),
        ("case_stage_values_canonical(case_stage_revisions)", "i"),
        ("case_stage_values_bytes(case_stage_revisions)", "i"),
        ("case_stage_recording_valid(case_stage_revisions)", "i"),
        ("preserve_case_stage_history()", "v"),
        ("enforce_case_stage_sequence()", "v"),
        ("preserve_case_stage_initial_exclusivity()", "v"),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_proc WHERE oid=to_regprocedure($1) AND NOT prosecdef AND provolatile::text=$2)",&[&signature,&volatility]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (name, target, columns, targets) in [
        ("case_stage_root_fk", "cases", vec!["case_id"], vec!["id"]),
        (
            "case_stage_administration_fk",
            "case_administration_revisions",
            vec!["case_id", "administration_revision"],
            vec!["case_id", "revision"],
        ),
        (
            "case_stage_actor_fk",
            "users",
            vec!["recorded_by"],
            vec!["id"],
        ),
        (
            "case_stage_support_fk",
            "documents",
            vec!["support_id", "support_version"],
            vec!["id", "version"],
        ),
        (
            "case_stage_receipt_fk",
            "documents",
            vec!["receipt_id", "receipt_version"],
            vec!["id", "version"],
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conrelid='case_stage_revisions'::regclass AND c.conname=$1 AND c.contype='f' AND c.convalidated AND c.confrelid=$2::text::regclass AND NOT c.condeferrable AND NOT c.condeferred AND c.confdeltype='a' AND c.confupdtype='a'
          AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[]
          AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$4::text[])",&[&name,&target,&columns,&targets]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let key:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conrelid='case_stage_revisions'::regclass AND c.contype='p' AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=ARRAY['case_id','revision'])",&[]).map_err(port)?.get(0);
    if !key {
        return Err(incomplete());
    }
    let checks = vec![
        "case_stage_revision_range",
        "case_stage_recording_range",
        "case_stage_actor_email",
        "case_stage_canonical",
        "case_stage_digest",
        "case_stage_recorded_after_acts",
    ];
    let valid:bool=client.query_one("SELECT count(*)=cardinality($1::text[]) FROM pg_catalog.pg_constraint WHERE conrelid='case_stage_revisions'::regclass AND contype='c' AND convalidated AND conname::text=ANY($1::text[])",&[&checks]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    for (table, name, kind, function) in [
        (
            TABLE,
            "case_stage_immutable",
            27i16,
            "preserve_case_stage_history()",
        ),
        (
            TABLE,
            "case_stage_sequence",
            7,
            "enforce_case_stage_sequence()",
        ),
        (
            "case_initial_stage_registrations",
            "case_stage_initial_exclusive",
            7,
            "preserve_case_stage_initial_exclusivity()",
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgtype=$3 AND tgfoid=$4::text::regprocedure AND tgenabled IN ('O','A') AND NOT tgdeferrable AND NOT tginitdeferred)",&[&table,&name,&kind,&function]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let enabled:bool=client.query_one("SELECT NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint WHERE c.conrelid='case_stage_revisions'::regclass AND t.tgenabled NOT IN ('O','A'))",&[]).map_err(port)?.get(0);
    if !enabled {
        return Err(incomplete());
    }
    validate_columns(client)
}
fn validate_columns(client: &mut Client) -> Result<(), ApplicationError> {
    let columns = [
        ("case_id", "uuid", true),
        ("revision", "bigint", true),
        ("change_kind", "text", true),
        ("from_stage", "text", false),
        ("stage", "text", true),
        ("administration_revision", "bigint", true),
        ("act_precision", "text", true),
        ("act_date", "date", false),
        ("act_seconds", "bigint", false),
        ("act_nanoseconds", "integer", false),
        ("act_offset_seconds", "integer", true),
        ("received_precision", "text", false),
        ("received_date", "date", false),
        ("received_seconds", "bigint", false),
        ("received_nanoseconds", "integer", false),
        ("received_offset_seconds", "integer", false),
        ("reason", "text", false),
        ("note", "text", false),
        ("receiving_court", "text", false),
        ("receipt_reference", "text", false),
        ("support_id", "uuid", true),
        ("support_version", "bigint", true),
        ("support_digest", "bytea", true),
        ("support_name", "text", true),
        ("support_format", "text", true),
        ("support_policy", "text", true),
        ("receipt_id", "uuid", false),
        ("receipt_version", "bigint", false),
        ("receipt_digest", "bytea", false),
        ("receipt_name", "text", false),
        ("receipt_format", "text", false),
        ("receipt_policy", "text", false),
        ("values_digest", "bytea", true),
        ("recorded_at_seconds", "bigint", true),
        ("recorded_at_nanoseconds", "integer", true),
        ("recorded_by", "uuid", true),
        ("recorded_by_email", "text", true),
    ];
    let count:i64=client.query_one("SELECT count(*) FROM pg_catalog.pg_attribute WHERE attrelid='case_stage_revisions'::regclass AND attnum>0 AND NOT attisdropped",&[]).map_err(port)?.get(0);
    if count != columns.len() as i64 {
        return Err(incomplete());
    }
    for (name, kind, required) in columns {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_attribute WHERE attrelid='case_stage_revisions'::regclass AND attname=$1 AND NOT attisdropped AND attnotnull=$3 AND NOT atthasdef AND pg_catalog.format_type(atttypid,atttypmod)=$2)",&[&name,&kind,&required]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    Ok(())
}
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "case stage schema is incomplete; run database migrate with an administrative role".into(),
    )
}
pub(super) fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "case stage inventory is inconsistent; restore a consistent database".into(),
    )
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("case stage schema: {error}"))
}
