use super::port;
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let quoted: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    client
        .batch_execute(&format!(
            "REVOKE ALL ON FUNCTION audit_timestamp_parts(TEXT) FROM {quoted};
        GRANT EXECUTE ON FUNCTION audit_timestamp_parts(TEXT) TO {quoted}"
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let missing: bool = client
        .query_one(
            "SELECT NOT has_table_privilege($1,'audit_events','SELECT')
        OR NOT has_table_privilege($1,'audit_events','INSERT')
        OR NOT has_function_privilege($1,'audit_timestamp_parts(text)','EXECUTE')",
            &[&role],
        )
        .map_err(port)?
        .get(0);
    let excessive: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_roles r
        WHERE pg_has_role($1,r.oid,'MEMBER') AND (r.rolsuper OR r.rolcreaterole OR r.rolbypassrls
        OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE c.oid='audit_events'::regclass AND (pg_has_role(r.oid,c.relowner,'MEMBER')
            OR pg_has_role(r.oid,n.nspowner,'MEMBER') OR has_schema_privilege(r.oid,n.oid,'CREATE')
            OR has_table_privilege(r.oid,c.oid,'UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES')
            OR has_any_column_privilege(r.oid,c.oid,'UPDATE,REFERENCES')))
        OR EXISTS(SELECT 1 FROM pg_proc p WHERE p.oid='audit_timestamp_parts(text)'::regprocedure
            AND pg_has_role(r.oid,p.proowner,'MEMBER'))))
        OR EXISTS(SELECT 1 FROM pg_class c,LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid='audit_events'::regclass AND
            (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))
        OR EXISTS(SELECT 1 FROM pg_attribute c,LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid='audit_events'::regclass AND
            (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))
        OR EXISTS(SELECT 1 FROM pg_proc p,LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid='audit_timestamp_parts(text)'::regprocedure AND
            (CASE WHEN a.grantee=0 THEN true ELSE a.is_grantable AND pg_has_role($1,a.grantee,'MEMBER') END))",
        &[&role]).map_err(port)?.get(0);
    if missing || excessive {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime audit query authority is unsafe or incomplete".into(),
        ));
    }
    Ok(())
}
