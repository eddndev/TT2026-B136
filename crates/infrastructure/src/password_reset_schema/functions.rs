use super::{function_specs::SPECS, incomplete, port, MIGRATIONS, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for spec in SPECS {
        let all: Vec<&str> = spec.inputs.iter().chain(spec.outputs).copied().collect();
        let row = client
            .query_opt(
                "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,p.prosecdef,
            p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
            p.pronamespace=c.relnamespace AND p.proowner=c.relowner,quote_ident(n.nspname),
            p.prokind::text,p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,
            p.prosupport::oid,p.proargtypes::oid[],
            coalesce(p.proallargtypes,ARRAY[]::oid[]),
            ARRAY(SELECT m::text FROM unnest(p.proargmodes) m),
            coalesce(p.proargnames,ARRAY[]::text[]),
            ARRAY(SELECT to_regtype(t)::oid FROM unnest($3::text[]) t),
            ARRAY(SELECT to_regtype(t)::oid FROM unnest($4::text[]) t)
            FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
            JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_class c ON c.oid=$2::text::regclass
            WHERE p.oid=to_regprocedure($1)",
                &[&spec.signature, &TABLE, &spec.inputs, &all],
            )
            .map_err(port)?
            .ok_or_else(incomplete)?;
        let name = spec.signature.split('(').next().ok_or_else(incomplete)?;
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
        let output = !spec.outputs.is_empty();
        let modes: Vec<&str> = if output {
            std::iter::repeat_n("i", spec.inputs.len())
                .chain(std::iter::repeat_n("t", spec.outputs.len()))
                .collect()
        } else {
            Vec::new()
        };
        let all_types: Vec<u32> = if output { row.get(21) } else { Vec::new() };
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != "v"
            || row.get::<_, String>(2) != spec.result
            || row.get::<_, bool>(3) != spec.definer
            || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
            || row.get::<_, bool>(5)
            || row.get::<_, bool>(6)
            || row.get::<_, String>(7) != "plpgsql"
            || !row.get::<_, bool>(8)
            || row.get::<_, String>(10) != "f"
            || row.get::<_, String>(11) != "u"
            || row.get::<_, bool>(12) != output
            || row.get::<_, i16>(13) != 0
            || row.get::<_, u32>(14) != 0
            || row.get::<_, u32>(15) != 0
            || row.get::<_, Vec<u32>>(16) != row.get::<_, Vec<u32>>(20)
            || row.get::<_, Vec<u32>>(17) != all_types
            || row.get::<_, Vec<String>>(18) != modes
            || row.get::<_, Vec<String>>(19) != spec.names
        {
            return Err(incomplete());
        }
    }
    let names: Vec<&str> = SPECS
        .iter()
        .filter_map(|s| s.signature.split('(').next())
        .collect();
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_proc p JOIN pg_class c ON c.oid=$1::text::regclass
        WHERE p.pronamespace=c.relnamespace AND p.proname::text=ANY($2::text[])",
            &[&TABLE, &names],
        )
        .map_err(port)?
        .get(0);
    if count != SPECS.len() as i64 {
        return Err(incomplete());
    }
    Ok(())
}

pub(super) fn expected_body(name: &str, schema: &str) -> Option<String> {
    let mut effective = None;
    for sql in MIGRATIONS {
        for definition in sql.split("CREATE OR REPLACE FUNCTION ").skip(1) {
            let (declaration, rest) = definition.split_once(" AS $function$")?;
            if declaration
                .strip_prefix("%1$I.")
                .unwrap_or(declaration)
                .starts_with(&format!("{name}("))
            {
                effective = rest
                    .split_once("$function$;")
                    .map(|(body, _)| body.replace("%%", "%").replace("%1$I", schema));
            }
        }
    }
    effective
}
