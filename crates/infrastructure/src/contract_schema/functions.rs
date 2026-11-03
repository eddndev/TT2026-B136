use super::{incomplete, port, MIGRATIONS, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

pub(super) const TRIGGERS: [&str; 10] = [
    "contract_lock()",
    "contract_preserve()",
    "contract_event_register()",
    "contract_request_register()",
    "contract_signature_register()",
    "contract_completion_register()",
    "contract_completion_pair()",
    "contract_notification_register()",
    "contract_notification_guard()",
    "contract_version_guard()",
];

/// Pure statement builders that the runtime role executes inside triggers.
pub(super) const BUILDERS: [(&str, &[&str], &[&str]); 2] = [
    (
        "contract_request_bytes(uuid,uuid,uuid,uuid,bigint,bytea,uuid,uuid,uuid,bytea,bytea,bigint,bigint)",
        &[
            "uuid", "uuid", "uuid", "uuid", "bigint", "bytea", "uuid", "uuid", "uuid", "bytea",
            "bytea", "bigint", "bigint",
        ],
        &[
            "deployment",
            "request",
            "case_id",
            "document",
            "version",
            "content",
            "client",
            "firm_signer",
            "requester",
            "client_leaf",
            "firm_leaf",
            "created_at",
            "expires_at",
        ],
    ),
    (
        "contract_party_bytes(uuid,bytea,bigint,uuid,bigint,bigint,uuid,bytea,uuid,uuid,bytea,bytea,smallint,bytea,bytea)",
        &[
            "uuid", "bytea", "bigint", "uuid", "bigint", "bigint", "uuid", "bytea", "uuid", "uuid",
            "bytea", "bytea", "smallint", "bytea", "bytea",
        ],
        &[
            "deployment",
            "root",
            "trust_revision",
            "account",
            "account_revision",
            "auth_generation",
            "binding",
            "leaf",
            "operation",
            "request",
            "request_digest",
            "content",
            "party",
            "signature_digest",
            "token_digest",
        ],
    ),
];

pub(super) fn builder_signatures() -> Vec<&'static str> {
    BUILDERS.iter().map(|builder| builder.0).collect()
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    for signature in TRIGGERS {
        function(client, signature, &[], &[], false)?;
    }
    for (signature, types, names) in BUILDERS {
        function(client, signature, types, names, true)?;
    }
    let names: Vec<&str> = TRIGGERS
        .iter()
        .chain(BUILDERS.iter().map(|builder| &builder.0))
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

/// A trigger function is volatile PL/pgSQL returning `trigger`; a builder is
/// an immutable, strict SQL function returning `bytea` with named arguments.
fn function<C: CatalogClient>(
    client: &mut C,
    signature: &str,
    types: &[&str],
    names: &[&str],
    builder: bool,
) -> Result<(), ApplicationError> {
    let row = client
        .query_opt(
            "SELECT p.prosrc,p.provolatile::text,p.prorettype::regtype::text,
                p.prosecdef,p.proconfig,p.proisstrict,p.proleakproof,l.lanname,
                n.oid=c.relnamespace AND p.proowner=c.relowner,quote_ident(n.nspname),p.prokind::text,
                p.proparallel::text,p.proretset,p.pronargdefaults,p.provariadic,
                ARRAY(SELECT t::oid FROM unnest(coalesce(p.proallargtypes,p.proargtypes::oid[])) t),
                ARRAY(SELECT m::text FROM unnest(p.proargmodes) m),
                coalesce(p.proargnames,ARRAY[]::text[]),p.prosupport::oid,
                ARRAY(SELECT to_regtype(t)::oid FROM unnest($3::text[]) t)
            FROM pg_proc p JOIN pg_language l ON l.oid=p.prolang
            JOIN pg_namespace n ON n.oid=p.pronamespace
            JOIN pg_class c ON c.oid=$2::text::regclass
            WHERE p.oid=to_regprocedure($1)",
            &[&signature, &TABLES[0], &types],
        )
        .map_err(port)?
        .ok_or_else(incomplete)?;
    let name = signature.split('(').next().ok_or_else(incomplete)?;
    let body = expected_body(name, &row.get::<_, String>(9)).ok_or_else(incomplete)?;
    let actual_types: Vec<u32> = row.get(15);
    let expected_types: Vec<u32> = row.get(19);
    let modes: Vec<String> = row.get(16);
    let actual_names: Vec<String> = row.get(17);
    let (volatility, returns, language) = if builder {
        ("i", "bytea", "sql")
    } else {
        ("v", "trigger", "plpgsql")
    };
    if row.get::<_, String>(0) != body
        || row.get::<_, String>(1) != volatility
        || row.get::<_, String>(2) != returns
        || row.get::<_, bool>(3)
        || row.get::<_, Option<Vec<String>>>(4) != Some(vec!["search_path=pg_catalog".into()])
        || row.get::<_, bool>(5) != builder
        || row.get::<_, bool>(6)
        || row.get::<_, String>(7) != language
        || !row.get::<_, bool>(8)
        || row.get::<_, String>(10) != "f"
        || row.get::<_, String>(11) != "u"
        || row.get::<_, bool>(12)
        || row.get::<_, i16>(13) != 0
        || row.get::<_, u32>(14) != 0
        || row.get::<_, u32>(18) != 0
        || actual_types != expected_types
        || !modes.is_empty()
        || actual_names != names
    {
        return Err(incomplete());
    }
    Ok(())
}

/// The body of the last registered definition of `name`, with the schema
/// placeholder of `format(... %1$I ...)` resolved.
fn expected_body(name: &str, schema: &str) -> Option<String> {
    let mut effective = None;
    for sql in MIGRATIONS {
        for definition in sql.split("CREATE OR REPLACE FUNCTION ").skip(1) {
            let Some((declaration, rest)) = definition.split_once(" AS $function$") else {
                continue;
            };
            let declaration = declaration.strip_prefix("%1$I.").unwrap_or(declaration);
            if declaration.starts_with(&format!("{name}(")) {
                effective = rest
                    .split_once("$function$;")
                    .map(|(body, _)| body.replace("%%", "%").replace("%1$I", schema));
            }
        }
    }
    effective
}

#[cfg(test)]
mod tests {
    use super::{expected_body, BUILDERS, TRIGGERS};

    #[test]
    fn every_function_has_one_registered_body_with_the_schema_resolved() {
        let names = TRIGGERS
            .iter()
            .chain(BUILDERS.iter().map(|builder| &builder.0))
            .filter_map(|signature| signature.split('(').next());
        for name in names {
            let body = expected_body(name, "tenant").unwrap();
            assert!(!body.contains("%1$I"), "{name}");
        }
        let register = expected_body("contract_signature_register", "tenant").unwrap();
        assert!(register.contains("FROM tenant.contract_events"));
        assert!(register.contains("tenant.contract_party_bytes("));
        let lock = expected_body("contract_lock", "tenant").unwrap();
        assert!(lock.contains("pg_advisory_xact_lock(280603412820)"));
    }
}
