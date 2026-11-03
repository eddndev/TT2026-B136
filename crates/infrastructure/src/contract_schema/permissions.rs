//! The runtime reads and appends contract history, executes the two statement
//! builders, and changes only the delivery columns of notifications.
use super::{columns, functions, incomplete, port, APPEND_ONLY, TABLES};
use crate::postgres::CatalogClient;
use crate::signing_binding_schema::permissions as shared;
use application::ApplicationError;
use postgres::GenericClient;

const INBOX: &str = TABLES[5];
const DELIVERY: [&str; 6] = [
    "delivery_state",
    "delivery_generation",
    "lease_id",
    "lease_until",
    "provider_deadline",
    "retry_at",
];

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    shared::grant_runtime(
        client,
        role,
        &APPEND_ONLY,
        columns::names,
        &functions::TRIGGERS,
        port,
    )?;
    let quoted: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    let names = columns::names(INBOX).join(",");
    let delivery = DELIVERY.join(",");
    let builders = functions::builder_signatures().join(",");
    client
        .batch_execute(&format!(
            "REVOKE ALL ON {INBOX} FROM {quoted};
            REVOKE SELECT({names}),INSERT({names}),UPDATE({names}),REFERENCES({names}) ON {INBOX} FROM {quoted};
            GRANT SELECT,INSERT ON {INBOX} TO {quoted};
            GRANT UPDATE({delivery}) ON {INBOX} TO {quoted};
            REVOKE ALL ON FUNCTION {builders} FROM {quoted};
            GRANT EXECUTE ON FUNCTION {builders} TO {quoted}"
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime<C: CatalogClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    shared::validate_runtime(
        client,
        role,
        &APPEND_ONLY,
        columns::names,
        &functions::TRIGGERS,
        port,
    )?;
    let names = columns::names(INBOX);
    let fixed: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| !DELIVERY.contains(name))
        .collect();
    let builders = functions::builder_signatures();
    // MEMBER includes roles reachable through SET ROLE even with NOINHERIT.
    let unsafe_role: bool = client.query_one(
        "SELECT NOT has_table_privilege($1,$2,'SELECT')
        OR EXISTS(SELECT 1 FROM unnest($3::text[]) a WHERE NOT has_column_privilege($1,$2,a,'INSERT'))
        OR EXISTS(SELECT 1 FROM unnest($4::text[]) a WHERE NOT has_column_privilege($1,$2,a,'UPDATE'))
        OR EXISTS(SELECT 1 FROM unnest($5::text[]) a WHERE has_column_privilege($1,$2,a,'UPDATE'))
        OR EXISTS(SELECT 1 FROM unnest($6::text[]) f WHERE NOT has_function_privilege($1,f,'EXECUTE'))
        OR EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
            r.rolsuper OR r.rolcreaterole OR r.rolbypassrls
            OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid=$2::text::regclass AND (
                    pg_has_role(r.oid,c.relowner,'MEMBER') OR pg_has_role(r.oid,n.nspowner,'MEMBER')
                    OR has_schema_privilege(r.oid,n.oid,'CREATE')
                    OR has_table_privilege(r.oid,c.oid,'UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES')
                    OR has_any_column_privilege(r.oid,c.oid,'REFERENCES')))
            OR EXISTS(SELECT 1 FROM pg_proc p WHERE p.oid IN (SELECT f::regprocedure FROM unnest($6::text[]) f)
                AND pg_has_role(r.oid,p.proowner,'MEMBER'))))
        OR EXISTS(SELECT 1 FROM pg_class c,
            LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid=$2::text::regclass AND (a.grantee=0 OR (a.is_grantable AND a.grantee<>c.relowner)))
        OR EXISTS(SELECT 1 FROM pg_attribute c,LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid=$2::text::regclass AND (a.grantee=0 OR a.is_grantable))
        OR EXISTS(SELECT 1 FROM pg_proc p,
            LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid IN (SELECT f::regprocedure FROM unnest($6::text[]) f)
                AND (a.grantee=0 OR (a.is_grantable AND a.grantee<>p.proowner)))",
        &[&role, &INBOX, &names, &DELIVERY.as_slice(), &fixed, &builders],
    ).map_err(port)?.get(0);
    if unsafe_role {
        return Err(incomplete());
    }
    Ok(())
}
