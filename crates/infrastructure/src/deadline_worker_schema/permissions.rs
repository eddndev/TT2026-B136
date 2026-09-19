use super::{columns, function_specs, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let role: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    for table in TABLES {
        let columns = columns::names(table).join(",");
        client
            .batch_execute(&format!(
                "REVOKE ALL ON {table} FROM {role};
            REVOKE INSERT({columns}),UPDATE({columns}),REFERENCES({columns}) ON {table} FROM {role};
            GRANT SELECT,INSERT({columns}) ON {table} TO {role}"
            ))
            .map_err(port)?;
    }
    let callable = function_specs::signatures(true).join(",");
    let triggers = function_specs::signatures(false).join(",");
    client
        .batch_execute(&format!(
            "REVOKE ALL ON FUNCTION {callable},{triggers} FROM {role};
        GRANT EXECUTE ON FUNCTION {callable} TO {role}"
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let callable = function_specs::signatures(true);
    let triggers = function_specs::signatures(false);
    for table in TABLES {
        let columns = columns::names(table);
        let missing: bool = client
            .query_one(
                "SELECT NOT has_table_privilege($1,$2,'SELECT')
                OR EXISTS(SELECT 1 FROM unnest($3::text[]) a
                    WHERE NOT has_column_privilege($1,$2,a,'INSERT'))",
                &[&role, &table, &columns],
            )
            .map_err(port)?
            .get(0);
        if missing {
            return Err(unsafe_role());
        }
    }
    let missing_function: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM unnest($2::text[]) f
            WHERE NOT has_function_privilege($1,f,'EXECUTE'))",
            &[&role, &callable],
        )
        .map_err(port)?
        .get(0);
    let unsafe_membership: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
            r.rolsuper OR r.rolcreaterole OR r.rolbypassrls
            OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid IN (SELECT t::regclass FROM unnest($2::text[]) t) AND (
                    pg_has_role(r.oid,c.relowner,'MEMBER') OR pg_has_role(r.oid,n.nspowner,'MEMBER')
                    OR has_schema_privilege(r.oid,n.oid,'CREATE')
                    OR has_table_privilege(r.oid,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES')
                    OR has_any_column_privilege(r.oid,c.oid,'UPDATE,REFERENCES')))
            OR EXISTS(SELECT 1 FROM pg_proc p
                WHERE p.oid IN (SELECT f::regprocedure FROM unnest($3::text[]||$4::text[]) f)
                    AND pg_has_role(r.oid,p.proowner,'MEMBER'))
            OR EXISTS(SELECT 1 FROM unnest($4::text[]) f
                WHERE has_function_privilege(r.oid,f,'EXECUTE'))))",
        &[&role, &&TABLES[..], &callable, &triggers],
    ).map_err(port)?.get(0);
    let public_privilege: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class c,
            LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND a.grantee=0)
        OR EXISTS(SELECT 1 FROM pg_attribute c,LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND a.grantee=0)
        OR EXISTS(SELECT 1 FROM pg_proc p,
            LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid IN (SELECT f::regprocedure FROM unnest($2::text[]||$3::text[]) f)
                AND a.grantee=0)",
            &[&&TABLES[..], &callable, &triggers],
        )
        .map_err(port)?
        .get(0);
    if missing_function || unsafe_membership || public_privilege {
        return Err(unsafe_role());
    }
    Ok(())
}

fn unsafe_role() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "runtime must only read and append deadline worker results and attempts through protected columns".into(),
    )
}
