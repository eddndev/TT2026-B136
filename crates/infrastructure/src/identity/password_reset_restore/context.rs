use super::{rejected, storage, PasswordResetRestoreRequest};
use application::ApplicationError;
use postgres::{Client, GenericClient, Transaction};

// Same schema exclusion as docs/adr/0048-schema-scoped-migration-lock.md.
const MIGRATION_LOCK_CLASS: i32 = 0x43415345;

#[derive(PartialEq, Eq)]
pub(super) struct Target {
    namespace: i32,
    relation: u32,
    owner: u32,
    namespace_owner: u32,
    quoted: String,
}

pub(super) fn pin_search_path(client: &mut Client, schema: &str) -> Result<(), ApplicationError> {
    client
        .query_one(
            "SELECT pg_catalog.set_config('search_path',
             pg_catalog.format('pg_catalog,%I,pg_temp',$1::pg_catalog.text),false)",
            &[&schema],
        )
        .map_err(|_| storage())?;
    Ok(())
}

pub(super) fn resolve<C: GenericClient>(
    client: &mut C,
    request: &PasswordResetRestoreRequest,
) -> Result<Target, ApplicationError> {
    let row = client.query_opt(
        "SELECT n.oid::pg_catalog.int4,c.oid,c.relowner,pg_catalog.quote_ident(n.nspname),n.nspowner
         FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
         WHERE c.oid=pg_catalog.to_regclass('password_reset_capabilities')
         AND n.nspname=$1 AND pg_catalog.current_database()=$2
         AND c.relowner=pg_catalog.to_regrole(current_user)::pg_catalog.oid
         AND c.relkind='r' AND c.relpersistence='p'
         AND NOT c.relispartition AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity",
        &[&request.expected_schema, &request.expected_database],
    ).map_err(|_| storage())?.ok_or_else(rejected)?;
    Ok(Target {
        namespace: row.get(0),
        relation: row.get(1),
        owner: row.get(2),
        quoted: row.get(3),
        namespace_owner: row.get(4),
    })
}

pub(super) fn lock(client: &mut Client, target: &Target) -> Result<(), ApplicationError> {
    client
        .query_one(
            "SELECT pg_catalog.pg_advisory_lock($1::pg_catalog.int4,$2::pg_catalog.int4)",
            &[&MIGRATION_LOCK_CLASS, &target.namespace],
        )
        .map_err(|_| storage())?;
    Ok(())
}

pub(super) fn validate(
    transaction: &mut Transaction<'_>,
    request: &PasswordResetRestoreRequest,
    target: &Target,
) -> Result<(), ApplicationError> {
    if resolve(transaction, request)? != *target {
        return Err(rejected());
    }
    transaction
        .query_one(
            "SELECT pg_catalog.set_config('search_path',$1,true)",
            &[&format!("pg_catalog,{},pg_temp", target.quoted)],
        )
        .map_err(|_| storage())?;
    if resolve(transaction, request)? != *target {
        return Err(rejected());
    }
    crate::password_reset_schema::validate_schema(transaction).map_err(|_| rejected())?;
    crate::member_schema::validate(transaction).map_err(|_| rejected())?;
    crate::audit_query_schema::validate(transaction).map_err(|_| rejected())?;
    validate_permissions(transaction)?;
    crate::member_schema::validate_inventory(transaction).map_err(|_| rejected())?;
    crate::password_reset_schema::validate_inventory(transaction).map_err(|_| rejected())?;
    crate::audit_query_schema::validate_inventory(transaction).map_err(|_| rejected())?;
    Ok(())
}

fn validate_permissions(transaction: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let rows = transaction
        .query(
            "SELECT DISTINCT r.rolname::pg_catalog.text FROM pg_catalog.pg_proc p
         CROSS JOIN LATERAL pg_catalog.aclexplode(
             coalesce(p.proacl,pg_catalog.acldefault('f',p.proowner))) a
         LEFT JOIN pg_catalog.pg_roles r ON r.oid=a.grantee
         WHERE p.oid='password_reset_issue(uuid,text,bytea,bigint,bigint)'::pg_catalog.regprocedure
         AND a.grantee<>p.proowner",
            &[],
        )
        .map_err(|_| storage())?;
    if rows.len() != 1 {
        return Err(rejected());
    }
    let role: String = rows[0].get::<_, Option<String>>(0).ok_or_else(rejected)?;
    crate::password_reset_schema::validate_runtime(transaction, &role).map_err(|_| rejected())?;
    crate::member_schema::validate_runtime_role(transaction, &role).map_err(|_| rejected())?;
    crate::audit_query_schema::validate_runtime_role(transaction, &role).map_err(|_| rejected())?;
    Ok(())
}
