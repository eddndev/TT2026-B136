use super::{function_specs::FUNCTIONS, incomplete, port, MIGRATIONS};
use application::ApplicationError;
use postgres::GenericClient;
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for spec in FUNCTIONS {
        let signature = spec.signature;
        let row=client.query_opt("SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,n.oid=c.relnamespace,quote_ident(n.nspname),p.prokind::text,p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_class c ON c.oid='case_deadlines'::regclass WHERE p.oid=to_regprocedure($1)",&[&signature]).map_err(port)?.ok_or_else(incomplete)?;
        let name = signature.split('(').next().ok_or_else(incomplete)?;
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
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
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
fn expected_body(name: &str, schema: &str) -> Option<String> {
    let mut effective = None;
    for sql in MIGRATIONS
        .iter()
        .chain(crate::deadline_worker_schema::MIGRATIONS.iter())
    {
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
