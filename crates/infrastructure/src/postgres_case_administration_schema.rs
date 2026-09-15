use application::ApplicationError;
use postgres::Client;

const VALUES_ARGS: &str = "text,text,text,text,text,text,text,text[],text,text";

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let tables = [
        "cases",
        "case_administration_revisions",
        "case_initial_stage_registrations",
    ];
    for table in tables {
        let present: bool = client
            .query_one("SELECT pg_catalog.to_regclass($1) IS NOT NULL", &[&table])
            .map_err(port)?
            .get(0);
        if !present {
            return Err(incomplete());
        }
    }
    for (name, volatility) in [
        (
            "case_administration_text_valid(text,integer,boolean)".to_owned(),
            "i",
        ),
        (
            format!("case_administration_is_canonical({VALUES_ARGS})"),
            "i",
        ),
        (format!("case_administration_bytes({VALUES_ARGS})"), "i"),
        ("preserve_case_administration_history()".to_owned(), "v"),
        ("enforce_case_administration_sequence()".to_owned(), "v"),
        ("validate_case_administration_heads()".to_owned(), "v"),
        ("validate_case_initial_stage_registration()".to_owned(), "v"),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_proc WHERE oid=pg_catalog.to_regprocedure($1) AND NOT prosecdef AND provolatile::text=$2)", &[&name,&volatility]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, key) in [
        ("cases", vec!["id"]),
        ("case_administration_revisions", vec!["case_id", "revision"]),
        ("case_initial_stage_registrations", vec!["case_id"]),
    ] {
        require_key(client, table, &key)?;
    }
    for (table, name, target, columns, targets, deferred) in [
        (
            "cases",
            "case_first_administration_revision_fk",
            "case_administration_revisions",
            vec!["id", "required_initial_revision"],
            vec!["case_id", "revision"],
            true,
        ),
        (
            "case_administration_revisions",
            "case_administration_root_fk",
            "cases",
            vec!["case_id"],
            vec!["id"],
            false,
        ),
        (
            "case_administration_revisions",
            "case_administration_actor_fk",
            "users",
            vec!["changed_by"],
            vec!["id"],
            false,
        ),
        (
            "case_initial_stage_registrations",
            "case_initial_stage_administration_fk",
            "case_administration_revisions",
            vec!["case_id", "administration_revision"],
            vec!["case_id", "revision"],
            false,
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='f' AND c.convalidated AND c.confrelid=$3::text::regclass AND c.condeferrable=$6 AND c.condeferred=$6 AND c.confdeltype='a' AND c.confupdtype='a'
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[]
            AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$5::text[])", &[&table,&name,&target,&columns,&targets,&deferred]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, names) in [
        (
            "cases",
            vec![
                "case_root_metadata_canonical",
                "case_created_at_finite",
                "case_required_initial_revision",
            ],
        ),
        (
            "case_administration_revisions",
            vec![
                "case_administration_revision_range",
                "case_administration_canonical",
                "case_administration_digest",
            ],
        ),
        (
            "case_initial_stage_registrations",
            vec![
                "case_initial_stage_revision",
                "case_initial_stage_administration_revision",
                "case_initial_stage_value",
            ],
        ),
    ] {
        let valid: bool = client.query_one("SELECT (SELECT count(*) FROM pg_catalog.pg_constraint WHERE conrelid=$1::text::regclass AND contype='c' AND convalidated AND conname::text=ANY($2::text[]))=cardinality($2::text[])", &[&table,&names]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, name, kind, function, deferred) in [
        (
            "cases",
            "case_root_immutable",
            19i16,
            "preserve_case_administration_history()",
            false,
        ),
        (
            "case_administration_revisions",
            "case_administration_immutable",
            19,
            "preserve_case_administration_history()",
            false,
        ),
        (
            "case_initial_stage_registrations",
            "case_initial_stage_immutable",
            19,
            "preserve_case_administration_history()",
            false,
        ),
        (
            "case_administration_revisions",
            "case_administration_sequence",
            7,
            "enforce_case_administration_sequence()",
            false,
        ),
        (
            "case_administration_revisions",
            "case_administration_heads_valid",
            5,
            "validate_case_administration_heads()",
            true,
        ),
        (
            "case_initial_stage_registrations",
            "case_initial_stage_valid",
            7,
            "validate_case_initial_stage_registration()",
            false,
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgtype=$3 AND tgfoid=$4::text::regprocedure AND tgenabled IN ('O','A') AND tgdeferrable=$5 AND tginitdeferred=$5)", &[&table,&name,&kind,&function,&deferred]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let enabled: bool = client.query_one("SELECT NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint WHERE c.conrelid=ANY($1::text[]::regclass[]) AND t.tgenabled NOT IN ('O','A'))", &[&&tables[..]]).map_err(port)?.get(0);
    if !enabled {
        return Err(incomplete());
    }
    validate_columns(client)
}

fn require_key(client: &mut Client, table: &str, key: &[&str]) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conrelid=$1::text::regclass AND c.contype='p' AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$2::text[])", &[&table,&key]).map_err(port)?.get(0);
    if valid {
        Ok(())
    } else {
        Err(incomplete())
    }
}

fn validate_columns(client: &mut Client) -> Result<(), ApplicationError> {
    for (table, columns) in [
        (
            "cases",
            vec![
                ("id", "uuid", true),
                ("title", "text", true),
                ("reference", "text", true),
                ("created_by", "uuid", true),
                ("created_at", "timestamp with time zone", true),
                ("required_initial_revision", "bigint", false),
            ],
        ),
        (
            "case_administration_revisions",
            vec![
                ("case_id", "uuid", true),
                ("revision", "bigint", true),
                ("title", "text", true),
                ("reference", "text", true),
                ("administrative_status", "text", true),
                ("nuc", "text", false),
                ("nuc_authority", "text", false),
                ("judicial_case_number", "text", false),
                ("judicial_authority", "text", false),
                ("offenses", "text[]", false),
                ("general_information", "text", false),
                ("complementary_identifiers", "text", false),
                ("values_digest", "bytea", true),
                ("changed_at", "text", true),
                ("changed_by", "uuid", true),
                ("changed_by_email", "text", true),
            ],
        ),
        (
            "case_initial_stage_registrations",
            vec![
                ("case_id", "uuid", true),
                ("stage_revision", "bigint", true),
                ("administration_revision", "bigint", true),
                ("stage", "text", true),
            ],
        ),
    ] {
        for (name, kind, required) in columns {
            let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_attribute WHERE attrelid=$1::text::regclass AND attname=$2 AND NOT attisdropped AND attnotnull=$4 AND pg_catalog.format_type(atttypid,atttypmod)=$3)", &[&table,&name,&kind,&required]).map_err(port)?.get(0);
            if !valid {
                return Err(incomplete());
            }
        }
    }
    let defaults: bool = client.query_one("SELECT (SELECT count(*) FROM pg_catalog.pg_attrdef d JOIN pg_catalog.pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum WHERE ((d.adrelid='cases'::regclass AND a.attname='required_initial_revision') OR (d.adrelid='case_initial_stage_registrations'::regclass AND a.attname IN ('stage_revision','administration_revision'))) AND pg_catalog.pg_get_expr(d.adbin,d.adrelid) IN ('1','1::bigint'))=3", &[]).map_err(port)?.get(0);
    if defaults {
        Ok(())
    } else {
        Err(incomplete())
    }
}

pub(super) fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("case administration schema is incomplete; run database migrate with an administrative role".into())
}
pub(super) fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "case administration inventory is inconsistent; restore a consistent database".into(),
    )
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("case administration schema: {error}"))
}
