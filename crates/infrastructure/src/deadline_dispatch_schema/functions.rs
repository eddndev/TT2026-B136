use super::{incomplete, port, CANDIDATES, MIGRATIONS, TRIGGER_FUNCTIONS};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let signatures: Vec<&str> = std::iter::once(CANDIDATES)
        .chain(TRIGGER_FUNCTIONS)
        .collect();
    for signature in &signatures {
        let row = client.query_opt(
            "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,
                p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
                n.oid=c.relnamespace,quote_ident(n.nspname),p.prokind::text,
                p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,
                ARRAY(SELECT format_type(t,NULL) FROM unnest(coalesce(p.proallargtypes,p.proargtypes::oid[])) t),
                ARRAY(SELECT m::text FROM unnest(p.proargmodes) m),
                coalesce(p.proargnames,ARRAY[]::text[]),p.prosupport::oid
            FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
            JOIN pg_namespace n ON n.oid=p.pronamespace
            JOIN pg_class c ON c.oid='deadline_dispatch_cursor'::regclass
            WHERE p.oid=to_regprocedure($1)",
            &[signature],
        ).map_err(port)?.ok_or_else(incomplete)?;
        let candidate = *signature == CANDIDATES;
        let name = signature.split('(').next().ok_or_else(incomplete)?;
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
        let argument_types: Vec<String> = row.get(15);
        let argument_modes: Vec<String> = row.get(16);
        let argument_names: Vec<String> = row.get(17);
        let arguments_valid = if candidate {
            argument_types
                == [
                    "bigint", "uuid", "boolean", "uuid", "boolean", "integer", "uuid", "uuid",
                ]
                && argument_modes == ["i", "i", "i", "i", "i", "i", "t", "t"]
                && argument_names
                    == [
                        "selected_event",
                        "lower_id",
                        "lower_inclusive",
                        "upper_id",
                        "missing_only",
                        "page_limit",
                        "deadline_id",
                        "case_id",
                    ]
        } else {
            argument_types.is_empty() && argument_modes.is_empty() && argument_names.is_empty()
        };
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != if candidate { "s" } else { "v" }
            || row.get::<_, String>(2) != if candidate { "record" } else { "trigger" }
            || row.get::<_, bool>(3)
            || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
            || row.get::<_, bool>(5)
            || row.get::<_, bool>(6)
            || row.get::<_, String>(7) != "plpgsql"
            || !row.get::<_, bool>(8)
            || row.get::<_, String>(10) != "f"
            || row.get::<_, String>(11) != "u"
            || row.get::<_, bool>(12) != candidate
            || row.get::<_, i16>(13) != 0
            || row.get::<_, u32>(14) != 0
            || row.get::<_, u32>(18) != 0
            || !arguments_valid
        {
            return Err(incomplete());
        }
    }
    let names: Vec<&str> = signatures
        .iter()
        .filter_map(|s| s.split('(').next())
        .collect();
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_proc p JOIN pg_class c
            ON c.oid='deadline_dispatch_cursor'::regclass
        WHERE p.pronamespace=c.relnamespace AND p.proname::text=ANY($1::text[])",
            &[&names],
        )
        .map_err(port)?
        .get(0);
    if count != 6 {
        return Err(incomplete());
    }
    Ok(())
}

fn expected_body(name: &str, schema: &str) -> Option<String> {
    let mut effective = None;
    for sql in MIGRATIONS {
        for definition in sql.split("CREATE OR REPLACE FUNCTION ").skip(1) {
            let Some((declaration, rest)) = definition.split_once(" AS $$") else {
                continue;
            };
            let declaration = declaration.strip_prefix("%1$I.").unwrap_or(declaration);
            if declaration.starts_with(&format!("{name}(")) {
                effective = rest
                    .split_once("$$;")
                    .map(|(body, _)| body.replace("%%", "%").replace("%1$I", schema));
            }
        }
    }
    effective
}
