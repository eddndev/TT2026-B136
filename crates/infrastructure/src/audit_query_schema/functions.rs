use super::{incomplete, port, MIGRATION};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let row = client
        .query_opt(
            "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,
        p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
        p.pronamespace=c.relnamespace AND p.proowner=c.relowner,p.prokind::text,
        p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,p.prosupport::oid,
        p.proargtypes::oid[],coalesce(p.proargnames,ARRAY[]::text[]),p.proargmodes IS NULL,
        p.proallargtypes IS NULL,ARRAY['text'::regtype::oid]
        FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
        JOIN pg_class c ON c.oid='audit_events'::regclass
        WHERE p.oid=to_regprocedure('audit_timestamp_parts(text)')",
            &[],
        )
        .map_err(port)?
        .ok_or_else(incomplete)?;
    let body = MIGRATION
        .split_once(" AS $function$")
        .and_then(|(_, rest)| rest.split_once("$function$;"))
        .map(|(body, _)| body)
        .ok_or_else(incomplete)?;
    if row.get::<_, String>(0) != body
        || row.get::<_, String>(1) != "i"
        || row.get::<_, String>(2) != "bigint[]"
        || row.get::<_, bool>(3)
        || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
        || !row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, String>(7) != "plpgsql"
        || !row.get::<_, bool>(8)
        || row.get::<_, String>(9) != "f"
        || row.get::<_, String>(10) != "s"
        || row.get::<_, bool>(11)
        || row.get::<_, i16>(12) != 0
        || row.get::<_, u32>(13) != 0
        || row.get::<_, u32>(14) != 0
        || row.get::<_, Vec<u32>>(15) != row.get::<_, Vec<u32>>(19)
        || row.get::<_, Vec<String>>(16) != ["value"]
        || !row.get::<_, bool>(17)
        || !row.get::<_, bool>(18)
    {
        return Err(incomplete());
    }
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_proc p JOIN pg_class c
        ON c.oid='audit_events'::regclass WHERE p.pronamespace=c.relnamespace
        AND p.proname='audit_timestamp_parts'",
            &[],
        )
        .map_err(port)?
        .get(0);
    if count != 1 {
        return Err(incomplete());
    }
    Ok(())
}
