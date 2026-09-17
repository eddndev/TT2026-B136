use super::{constraints, incomplete, port, HELPERS, TABLES, TRIGGERS};
use application::ApplicationError;
use postgres::GenericClient;

const SQL: [&str; 10] = [
    include_str!("../../../../migrations/0014_procedural_fact_primitives.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_time.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_people.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_provenance.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_values.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_items.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_sources.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_receipts.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_guards.sql"),
    include_str!("../../../../migrations/0014_procedural_facts_guards.sql"),
];
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass($1) AND relkind='r'
             AND relpersistence='p' AND NOT relispartition AND NOT relrowsecurity AND NOT relforcerowsecurity
             AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=to_regclass($1) OR inhparent=to_regclass($1)))",
            &[&table],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    columns(
        client,
        TABLES[0],
        &[
            ("family", "text", true, ""),
            ("id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("parent_resolution_id", "uuid", false, ""),
            ("parent_family", "text", false, "s"),
            ("initial_revision", "bigint", true, ""),
        ],
    )?;
    columns(
        client,
        TABLES[1],
        &[
            ("family", "text", true, ""),
            ("id", "uuid", true, ""),
            ("case_id", "uuid", true, ""),
            ("revision", "bigint", true, ""),
            ("values_canonical", "bytea", true, ""),
            ("values_digest", "bytea", true, ""),
            ("values_view", "jsonb", false, "s"),
            ("sources_canonical", "bytea", true, ""),
            ("sources_digest", "bytea", true, ""),
            ("sources_view", "jsonb", false, "s"),
            ("operation_id", "uuid", true, ""),
            ("action", "text", true, ""),
            ("status", "text", false, "s"),
            ("reason", "text", false, ""),
            ("submission_canonical", "bytea", true, ""),
            ("submission_digest", "bytea", true, ""),
            ("submission_view", "jsonb", false, "s"),
            ("recorded_administration_revision", "bigint", false, ""),
            ("recorded_administration_digest", "bytea", false, ""),
            ("recorded_administration_title", "text", false, ""),
            ("recorded_administration_reference", "text", false, ""),
            ("recorded_at_seconds", "bigint", true, ""),
            ("recorded_at_nanoseconds", "integer", true, ""),
            ("recorded_by", "uuid", true, ""),
            ("recorded_by_email", "text", true, ""),
        ],
    )?;
    constraints::validate(client)?;
    functions(client)?;
    for (table, trigger, function, kind) in [
        (TABLES[0], "procedural_fact_immutable", TRIGGERS[0], 58_i16),
        (TABLES[1], "procedural_fact_immutable", TRIGGERS[0], 58),
        (TABLES[0], "procedural_fact_root_sources", TRIGGERS[1], 7),
        (TABLES[1], "procedural_fact_sequence", TRIGGERS[2], 7),
        (
            TABLES[1],
            "deadline_source_emit",
            "emit_deadline_source_event()",
            5,
        ),
    ] {
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgname=$2
             AND tgfoid=$3::text::regprocedure AND tgtype=$4 AND tgenabled IN ('O','A')
             AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred
             AND tgqual IS NULL AND tgnargs=0 AND tgattr=''::int2vector)",
            &[&table, &trigger, &function, &kind],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let altered: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid IN
            (SELECT t::regclass FROM unnest($1::text[]) t) AND tgenabled NOT IN ('O','A'))
         OR (SELECT count(*) FROM pg_trigger WHERE tgrelid IN
            (SELECT t::regclass FROM unnest($1::text[]) t) AND NOT tgisinternal)<>5",
            &[&&TABLES[..]],
        )
        .map_err(port)?
        .get(0);
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
    let rows = client.query(
        "SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,attgenerated::text,attidentity::text
         FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped",
        &[&table],
    ).map_err(port)?;
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
fn functions<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for signature in HELPERS.into_iter().chain(TRIGGERS) {
        let row = client.query_opt(
            "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,p.prosecdef,p.proconfig,
             p.proisstrict,p.proleakproof,l.lanname,n.oid=c.relnamespace,quote_ident(n.nspname),
             p.prokind::text,p.proparallel::text FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
             JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_class c ON c.oid='case_procedural_facts'::regclass
             WHERE p.oid=to_regprocedure($1)", &[&signature],
        ).map_err(port)?.ok_or_else(incomplete)?;
        let name = signature.split('(').next().ok_or_else(incomplete)?;
        let validator = signature == HELPERS[10];
        let helper = HELPERS.contains(&signature);
        let volatility = if helper && !validator { "i" } else { "v" };
        let returns = if validator {
            "void"
        } else if helper {
            "jsonb"
        } else {
            "trigger"
        };
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != volatility
            || row.get::<_, String>(2) != returns
            || row.get::<_, bool>(3)
            || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
            || row.get::<_, bool>(5)
            || row.get::<_, bool>(6)
            || row.get::<_, String>(7) != "plpgsql"
            || !row.get::<_, bool>(8)
            || row.get::<_, String>(10) != "f"
            || row.get::<_, String>(11) != "u"
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
fn expected_body(name: &str, schema: &str) -> Option<String> {
    for sql in SQL {
        for definition in sql.split("CREATE OR REPLACE FUNCTION ").skip(1) {
            let (declaration, rest) = definition.split_once(" AS $$")?;
            let declaration = declaration.strip_prefix("%1$I.").unwrap_or(declaration);
            if declaration.starts_with(&format!("{name}(")) {
                return rest
                    .split_once("$$;")
                    .map(|(body, _)| body.replace("%%", "%").replace("%1$I", schema));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn expected_body_preserves_percent_signs_inside_schema_identifiers() {
        let body = super::expected_body("procedural_fact_time", "\"facts%%archive\"").unwrap();
        assert!(body.contains("\"facts%%archive\".typed_u32"));
        assert!(body.contains("offset_value % 60"));
        assert!(!body.contains("%1$I"));
    }
}
