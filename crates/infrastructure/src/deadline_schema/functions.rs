use super::{incomplete, port, HELPERS, TRIGGERS};
use application::ApplicationError;
use postgres::GenericClient;
const SQL: [&str; 4] = [
    include_str!("../../../../migrations/0017_deadline_receipts.sql"),
    include_str!("../../../../migrations/0017_deadline_attention.sql"),
    include_str!("../../../../migrations/0017_deadline_input_selection.sql"),
    include_str!("../../../../migrations/0017_deadline_guards.sql"),
];
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for signature in HELPERS.into_iter().chain(TRIGGERS) {
        let row=client.query_opt("SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,n.oid=c.relnamespace,quote_ident(n.nspname),p.prokind::text,p.proparallel::text FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_class c ON c.oid='case_deadlines'::regclass WHERE p.oid=to_regprocedure($1)",&[&signature]).map_err(port)?.ok_or_else(incomplete)?;
        let name = signature.split('(').next().ok_or_else(incomplete)?;
        let helper = HELPERS.contains(&signature);
        let returns = if signature == "deadline_attention_valid(jsonb)" {
            "boolean"
        } else if helper {
            "jsonb"
        } else {
            "trigger"
        };
        let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != if helper { "i" } else { "v" }
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
