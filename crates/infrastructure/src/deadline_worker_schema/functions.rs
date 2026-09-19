use super::{function_specs::FUNCTIONS, incomplete, port, MIGRATIONS, RESULTS};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for spec in FUNCTIONS {
        let row = client
            .query_opt(
                "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,
                p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
                n.oid=c.relnamespace,quote_ident(n.nspname),p.prokind::text,
                p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,
                ARRAY(SELECT t::oid FROM unnest(coalesce(p.proallargtypes,p.proargtypes::oid[])) t),
                ARRAY(SELECT m::text FROM unnest(p.proargmodes) m),
                coalesce(p.proargnames,ARRAY[]::text[]),p.prosupport::oid,
                ARRAY(SELECT to_regtype(t)::oid FROM unnest($3::text[]) t)
            FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
            JOIN pg_namespace n ON n.oid=p.pronamespace
            JOIN pg_class c ON c.oid=$2::text::regclass
            WHERE p.oid=to_regprocedure($1)",
                &[&spec.signature, &RESULTS, &spec.types],
            )
            .map_err(port)?
            .ok_or_else(incomplete)?;
        let name = spec.signature.split('(').next().ok_or_else(incomplete)?;
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
        let types: Vec<u32> = row.get(15);
        let expected_types: Vec<u32> = row.get(19);
        let modes: Vec<String> = row.get(16);
        let names: Vec<String> = row.get(17);
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != spec.volatility
            || row.get::<_, String>(2) != spec.returns
            || row.get::<_, bool>(3)
            || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
            || row.get::<_, bool>(5)
            || row.get::<_, bool>(6)
            || row.get::<_, String>(7) != "plpgsql"
            || !row.get::<_, bool>(8)
            || row.get::<_, String>(10) != "f"
            || row.get::<_, String>(11) != "u"
            || row.get::<_, bool>(12)
            || row.get::<_, i16>(13) != 0
            || row.get::<_, u32>(14) != 0
            || row.get::<_, u32>(18) != 0
            || types != expected_types
            || !modes.is_empty()
            || names != spec.names
        {
            return Err(incomplete());
        }
    }
    let names: Vec<&str> = FUNCTIONS
        .iter()
        .filter_map(|spec| spec.signature.split('(').next())
        .collect();
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_proc p JOIN pg_class c ON c.oid=$1::text::regclass
        WHERE p.pronamespace=c.relnamespace AND p.proname::text=ANY($2::text[])",
            &[&RESULTS, &names],
        )
        .map_err(port)?
        .get(0);
    if count != i64::try_from(FUNCTIONS.len()).map_err(|_| incomplete())? {
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
