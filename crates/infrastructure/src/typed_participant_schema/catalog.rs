use super::{constraints, incomplete, port, HELPERS, TABLES, TRIGGERS};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let present: bool = client
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .map_err(port)?
            .get(0);
        if !present {
            return Err(incomplete());
        }
    }
    columns(
        client,
        "case_subjects",
        &[
            ("id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        "case_subject_revisions",
        &[
            ("subject_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("values_canonical", "bytea", true, ""),
            ("values_digest", "bytea", true, ""),
            ("values_view", "jsonb", false, "s"),
            ("subject_kind", "text", true, ""),
            ("display_name", "text", true, ""),
            ("name_known", "boolean", true, ""),
            ("declared_identifier", "text", false, ""),
            ("identity_document_id", "uuid", true, ""),
            ("identity_document_version", "bigint", true, ""),
            ("identity_document_digest", "bytea", true, ""),
            ("identity_document_locator", "text", true, ""),
            ("changed_at", "text", true, ""),
            ("changed_by", "uuid", true, ""),
            ("changed_by_email", "text", true, ""),
        ],
    )?;
    columns(
        client,
        "case_participant_typed_revisions",
        &[
            ("participant_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("values_canonical", "bytea", true, ""),
            ("values_digest", "bytea", true, ""),
            ("values_view", "jsonb", false, "s"),
            ("subject_id", "uuid", true, ""),
            ("subject_revision", "bigint", true, ""),
            ("role_kind", "text", true, ""),
            ("organization", "text", false, ""),
            ("directory_status", "text", true, ""),
            ("changed_at", "text", true, ""),
            ("changed_by", "uuid", true, ""),
            ("changed_by_email", "text", true, ""),
            ("submission_digest", "bytea", true, ""),
            ("submission_revision", "bigint", true, ""),
            ("credential_origin_revision", "bigint", false, ""),
        ],
    )?;
    columns(
        client,
        "subject_identity_reviews",
        &[
            ("subject_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("review_canonical", "bytea", true, ""),
            ("review_digest", "bytea", true, ""),
        ],
    )?;
    columns(
        client,
        "participant_identity_reviews",
        &[
            ("participant_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("review_canonical", "bytea", true, ""),
            ("review_digest", "bytea", true, ""),
            ("submission_canonical", "bytea", true, ""),
            ("submission_digest", "bytea", true, ""),
        ],
    )?;
    columns(
        client,
        "participant_credential_evidence",
        &[
            ("participant_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("deployment_id", "uuid", true, ""),
            ("trust_revision", "bigint", true, ""),
            ("declaration", "bytea", true, ""),
            ("statement_digest", "bytea", true, ""),
            ("certificate_der", "bytea", true, ""),
            ("certificate_fingerprint", "bytea", true, ""),
            ("signature", "bytea", true, ""),
            ("checked_at", "bigint", true, ""),
            ("valid_from", "bigint", true, ""),
            ("valid_until", "bigint", true, ""),
            ("accepted_at_seconds", "bigint", true, ""),
            ("accepted_at_nanoseconds", "integer", true, ""),
        ],
    )?;
    for (table, function) in [
        ("case_subject_revisions", "typed_subject_values(bytea)"),
        (
            "case_participant_typed_revisions",
            "typed_participant_values(bytea)",
        ),
    ] {
        let bound: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_attrdef a
            JOIN pg_catalog.pg_attribute c ON c.attrelid=a.adrelid AND c.attnum=a.adnum
            JOIN pg_catalog.pg_depend d ON d.classid='pg_attrdef'::regclass AND d.objid=a.oid
            WHERE a.adrelid=$1::text::regclass AND c.attname='values_view'
                AND d.refclassid='pg_proc'::regclass AND d.refobjid=to_regprocedure($2))",
                &[&table, &function],
            )
            .map_err(port)?
            .get(0);
        if !bound {
            return Err(incomplete());
        }
    }
    constraints::validate(client)?;
    for (index, function) in HELPERS.iter().enumerate() {
        let returns = match index {
            0 => "bigint",
            1..=4 => "jsonb",
            5 => "bytea",
            _ => "void",
        };
        function_kind(client, function, if index < 5 { "i" } else { "v" }, returns)?;
    }
    for function in TRIGGERS {
        function_kind(client, function, "v", "trigger")?;
    }
    for table in TABLES {
        trigger(
            client,
            table,
            "typed_immutable",
            27,
            "preserve_typed_history()",
            false,
        )?;
    }
    for (table, name, function, deferred) in [
        (
            "case_subject_revisions",
            "subject_sequence",
            "enforce_subject_sequence()",
            false,
        ),
        (
            "case_participant_typed_revisions",
            "typed_sequence",
            "enforce_typed_sequence()",
            false,
        ),
        (
            "case_subjects",
            "subject_root_complete",
            "typed_root_complete()",
            true,
        ),
        (
            "case_participants",
            "typed_root_complete",
            "typed_root_complete()",
            true,
        ),
        (
            "subject_identity_reviews",
            "subject_review_guard",
            "typed_review_guard()",
            true,
        ),
        (
            "participant_identity_reviews",
            "participant_review_guard",
            "typed_review_guard()",
            true,
        ),
        (
            "participant_credential_evidence",
            "participant_credential_guard",
            "typed_credential_guard()",
            true,
        ),
    ] {
        trigger(
            client,
            table,
            name,
            if deferred { 5 } else { 7 },
            function,
            deferred,
        )?;
    }
    let disabled:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t
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
            "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_proc WHERE oid=to_regprocedure($1)
        AND NOT prosecdef AND provolatile::text=$2 AND prorettype=$3::text::regtype)",
            &[&name, &volatility, &returns],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
fn trigger<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    kind: i16,
    function: &str,
    deferred: bool,
) -> Result<(), ApplicationError> {
    let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger WHERE tgrelid=$1::text::regclass
        AND tgname=$2 AND tgtype=$3 AND tgfoid=to_regprocedure($4) AND tgdeferrable=$5 AND tginitdeferred=$5
        AND tgenabled IN ('O','A'))",&[&table,&name,&kind,&function,&deferred]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
