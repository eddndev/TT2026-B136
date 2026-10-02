use super::{columns::COLUMNS, function_specs::SPECS, port, TABLE};
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
    let columns = COLUMNS
        .iter()
        .map(|column| column.0)
        .collect::<Vec<_>>()
        .join(",");
    let all = SPECS
        .iter()
        .map(|spec| spec.signature)
        .collect::<Vec<_>>()
        .join(",");
    let allowed = SPECS
        .iter()
        .filter(|spec| spec.definer)
        .map(|spec| spec.signature)
        .collect::<Vec<_>>()
        .join(",");
    client.batch_execute(&format!(
        "REVOKE ALL ON {TABLE} FROM {quoted};
        REVOKE SELECT({columns}),INSERT({columns}),UPDATE({columns}),REFERENCES({columns}) ON {TABLE} FROM {quoted};
        REVOKE ALL ON FUNCTION {all} FROM {quoted};
        GRANT EXECUTE ON FUNCTION {allowed} TO {quoted};"
    )).map_err(port)
}

pub(crate) fn validate_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let allowed: Vec<&str> = SPECS
        .iter()
        .filter(|spec| spec.definer)
        .map(|spec| spec.signature)
        .collect();
    let all: Vec<&str> = SPECS.iter().map(|spec| spec.signature).collect();
    let unsafe_role: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM unnest($3::text[]) f WHERE NOT has_function_privilege($1,f,'EXECUTE'))
        OR has_function_privilege($1,'password_reset_audit_receipt()','EXECUTE')
        OR EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND
            (r.rolsuper OR r.rolcreaterole OR r.rolbypassrls
            OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid=$2::text::regclass AND (pg_has_role(r.oid,c.relowner,'MEMBER')
                    OR pg_has_role(r.oid,n.nspowner,'MEMBER') OR has_schema_privilege(r.oid,n.oid,'CREATE')
                    OR has_table_privilege(r.oid,c.oid,'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
                    OR has_any_column_privilege(r.oid,c.oid,'SELECT,INSERT,UPDATE,REFERENCES')))
            OR EXISTS(SELECT 1 FROM pg_proc p WHERE p.oid IN
                (SELECT f::regprocedure FROM unnest($4::text[]) f) AND pg_has_role(r.oid,p.proowner,'MEMBER'))))
        OR EXISTS(SELECT 1 FROM pg_class c,
            LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid=$2::text::regclass AND a.grantee<>c.relowner)
        OR EXISTS(SELECT 1 FROM pg_attribute c JOIN pg_class t ON t.oid=c.attrelid,
            LATERAL aclexplode(c.attacl) a WHERE c.attrelid=$2::text::regclass AND a.grantee<>t.relowner)
        OR EXISTS(SELECT 1 FROM pg_proc p,
            LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid IN (SELECT f::regprocedure FROM unnest($4::text[]) f)
            AND a.grantee<>p.proowner AND (a.grantee=0 OR a.is_grantable
                OR a.grantee<>to_regrole($1)::oid OR a.privilege_type<>'EXECUTE'
                OR p.oid='password_reset_audit_receipt()'::regprocedure))",
        &[&role, &TABLE, &allowed, &all],
    ).map_err(port)?.get(0);
    if unsafe_role {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime password reset authority is unsafe or incomplete".into(),
        ));
    }
    Ok(())
}
