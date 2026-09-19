use super::{columns, functions::TRIGGERS, port, TABLES};
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
        let names = columns::names(table).join(",");
        client.batch_execute(&format!(
            "REVOKE ALL ON {table} FROM {role};
            REVOKE SELECT({names}),INSERT({names}),UPDATE({names}),REFERENCES({names}) ON {table} FROM {role};
            GRANT SELECT ON {table} TO {role}"
        )).map_err(port)?;
        client
            .batch_execute(&format!("GRANT INSERT({names}) ON {table} TO {role}"))
            .map_err(port)?;
    }
    let triggers = TRIGGERS.join(",");
    client
        .batch_execute(&format!("REVOKE ALL ON FUNCTION {triggers} FROM {role}"))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    for table in TABLES {
        let insert = columns::names(table);
        let missing: bool = client.query_one(
            "SELECT NOT has_table_privilege($1,$2,'SELECT')
                OR EXISTS(SELECT 1 FROM unnest($3::text[]) a WHERE NOT has_column_privilege($1,$2,a,'INSERT'))",
            &[&role, &table, &insert]
        ).map_err(port)?.get(0);
        // MEMBER includes roles reachable through SET ROLE even with NOINHERIT.
        let excessive: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
                r.rolsuper OR r.rolcreaterole OR r.rolbypassrls
                OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                    WHERE c.oid=$2::text::regclass AND (
                        pg_has_role(r.oid,c.relowner,'MEMBER') OR pg_has_role(r.oid,n.nspowner,'MEMBER')
                        OR has_schema_privilege(r.oid,n.oid,'CREATE')
                        OR has_table_privilege(r.oid,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES')
                        OR has_any_column_privilege(r.oid,c.oid,'UPDATE,REFERENCES')))))",
            &[&role, &table]
        ).map_err(port)?.get(0);
        if missing || excessive {
            return Err(unsafe_role());
        }
    }
    let unsafe_function: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
            EXISTS(SELECT 1 FROM pg_proc p
                WHERE p.oid IN (SELECT f::regprocedure FROM unnest($2::text[]) f)
                    AND pg_has_role(r.oid,p.proowner,'MEMBER'))
            OR EXISTS(SELECT 1 FROM unnest($2::text[]) f WHERE has_function_privilege(r.oid,f,'EXECUTE'))))",
        &[&role, &&TRIGGERS[..]]
    ).map_err(port)?.get(0);
    if unsafe_function || exposed_acl(client, role)? {
        return Err(unsafe_role());
    }
    Ok(())
}

fn exposed_acl<C: GenericClient>(client: &mut C, role: &str) -> Result<bool, ApplicationError> {
    client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_class c,
            LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid IN (SELECT t::regclass FROM unnest($2::text[]) t)
                AND (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))
        OR EXISTS(SELECT 1 FROM pg_attribute c,LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid IN (SELECT t::regclass FROM unnest($2::text[]) t)
                AND (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))
        OR EXISTS(SELECT 1 FROM pg_proc p,
            LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid IN (SELECT f::regprocedure FROM unnest($3::text[]) f)
                AND (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))",
        &[&role, &&TABLES[..], &&TRIGGERS[..]]
    ).map_err(port).map(|row| row.get(0))
}

fn unsafe_role() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "runtime must only read and append resource activities without public or delegated authority".into(),
    )
}
