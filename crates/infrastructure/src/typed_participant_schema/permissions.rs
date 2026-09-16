use application::ApplicationError;
use postgres::GenericClient;

use super::{port, HELPERS, TABLES, TRIGGERS};

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let role: String = client
        .query_one("SELECT pg_catalog.quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    let tables = TABLES.join(",");
    client
        .batch_execute(&format!(
            "REVOKE ALL ON {tables} FROM {role}; GRANT SELECT,INSERT ON {tables} TO {role};
        REVOKE ALL ON FUNCTION {} FROM {role}; GRANT EXECUTE ON FUNCTION {} TO {role}",
            TRIGGERS.join(","),
            HELPERS.join(",")
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let unsafe_role:bool=client.query_one("SELECT
        EXISTS(SELECT 1 FROM unnest($2::text[]) t WHERE NOT pg_catalog.has_table_privilege($1,t,'SELECT')
            OR NOT pg_catalog.has_table_privilege($1,t,'INSERT'))
        OR EXISTS(SELECT 1 FROM unnest($3::text[]) f WHERE NOT pg_catalog.has_function_privilege($1,f,'EXECUTE'))
        OR EXISTS(SELECT 1 FROM pg_catalog.pg_roles r WHERE pg_catalog.pg_has_role($1,r.oid,'MEMBER') AND (
            r.rolsuper OR r.rolcreaterole OR EXISTS(SELECT 1 FROM pg_catalog.pg_class c
                JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid IN (SELECT t::regclass FROM unnest($2::text[]) t) AND (
                    pg_catalog.pg_has_role(r.oid,c.relowner,'MEMBER') OR pg_catalog.pg_has_role(r.oid,n.nspowner,'MEMBER')
                    OR pg_catalog.has_table_privilege(r.oid,c.oid,'UPDATE,DELETE,TRUNCATE,TRIGGER')
                    OR pg_catalog.has_any_column_privilege(r.oid,c.oid,'UPDATE')))
            OR EXISTS(SELECT 1 FROM pg_catalog.pg_proc p
                WHERE p.oid IN (SELECT f::regprocedure FROM unnest($3::text[]||$4::text[]) f)
                    AND pg_catalog.pg_has_role(r.oid,p.proowner,'MEMBER'))
            OR EXISTS(SELECT 1 FROM unnest($4::text[]) f WHERE pg_catalog.has_function_privilege(r.oid,f,'EXECUTE'))))",
        &[&role,&&TABLES[..],&&HELPERS[..],&&TRIGGERS[..]]).map_err(port)?.get(0);
    if unsafe_role {
        return Err(ApplicationError::InvalidConfiguration(
        "runtime database role must only read and append typed participant history without owning protected objects".into()));
    }
    Ok(())
}
