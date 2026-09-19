use super::{incomplete, port, MIGRATIONS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) const CALLABLE: [&str; 3] = [
    "alert_time_valid(bigint,integer)",
    "alert_optional_time_valid(bigint,integer)",
    "alert_payload_valid(bytea,bytea)",
];
pub(super) const TRIGGERS: [&str; 5] = [
    "lock_alert_mutations()",
    "preserve_alert_rows()",
    "validate_alert_update()",
    "validate_alert_attempt()",
    "validate_alert_outbox_ledger()",
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for signature in CALLABLE.into_iter().chain(TRIGGERS) {
        let callable = CALLABLE.contains(&signature);
        let payload = signature.starts_with("alert_payload_valid(");
        let types: &[&str] = if payload {
            &["bytea", "bytea"]
        } else if callable {
            &["bigint", "integer"]
        } else {
            &[]
        };
        let names: &[&str] = if payload {
            &["payload", "digest"]
        } else if callable {
            &["seconds", "nanos"]
        } else {
            &[]
        };
        let strict = callable && !signature.starts_with("alert_optional_time_valid(");
        let row = client
            .query_opt(
                "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,
                p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
                p.pronamespace=c.relnamespace AND p.proowner=c.relowner,p.prokind::text,
                p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,
                ARRAY(SELECT t::oid FROM unnest(coalesce(p.proallargtypes,p.proargtypes::oid[])) t),
                ARRAY(SELECT m::text FROM unnest(p.proargmodes) m),
                coalesce(p.proargnames,ARRAY[]::text[]),p.prosupport::oid,
                ARRAY(SELECT to_regtype(t)::oid FROM unnest($3::text[]) t)
            FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
            JOIN pg_class c ON c.oid=$2::text::regclass WHERE p.oid=to_regprocedure($1)",
                &[&signature, &TABLES[0], &types],
            )
            .map_err(port)?
            .ok_or_else(incomplete)?;
        let name = signature.split('(').next().ok_or_else(incomplete)?;
        let body = expected_body(name).ok_or_else(incomplete)?;
        if row.get::<_, String>(0) != body
            || row.get::<_, String>(1) != if callable { "i" } else { "v" }
            || row.get::<_, String>(2) != if callable { "boolean" } else { "trigger" }
            || row.get::<_, bool>(3)
            || row.get::<_, Option<Vec<String>>>(4).is_some()
            || row.get::<_, bool>(5) != strict
            || row.get::<_, bool>(6)
            || row.get::<_, String>(7) != if callable { "sql" } else { "plpgsql" }
            || !row.get::<_, bool>(8)
            || row.get::<_, String>(9) != "f"
            || row.get::<_, String>(10) != if callable { "s" } else { "u" }
            || row.get::<_, bool>(11)
            || row.get::<_, i16>(12) != 0
            || row.get::<_, u32>(13) != 0
            || row.get::<_, u32>(17) != 0
            || row.get::<_, Vec<u32>>(14) != row.get::<_, Vec<u32>>(18)
            || !row.get::<_, Vec<String>>(15).is_empty()
            || row.get::<_, Vec<String>>(16) != names
        {
            return Err(incomplete());
        }
    }
    let names: Vec<&str> = CALLABLE
        .into_iter()
        .chain(TRIGGERS)
        .filter_map(|signature| signature.split('(').next())
        .collect();
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_proc p JOIN pg_class c ON c.oid=$1::text::regclass
        WHERE p.pronamespace=c.relnamespace AND p.proname::text=ANY($2::text[])",
            &[&TABLES[0], &names],
        )
        .map_err(port)?
        .get(0);
    if usize::try_from(count).ok() != Some(names.len()) {
        return Err(incomplete());
    }
    Ok(())
}

fn expected_body(name: &str) -> Option<&'static str> {
    MIGRATIONS
        .iter()
        .flat_map(|sql| sql.split("CREATE OR REPLACE FUNCTION ").skip(1))
        .filter_map(|definition| definition.split_once(" AS $$"))
        .find(|(declaration, _)| declaration.starts_with(&format!("{name}(")))
        .and_then(|(_, rest)| rest.split_once("$$;").map(|(body, _)| body))
}
