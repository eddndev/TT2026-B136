use super::{invalid, port};
use application::ApplicationError;
use postgres::Client;

pub(super) fn validate(client: &mut Client, role: &str) -> Result<(), ApplicationError> {
    let permissions = client
        .query_one(
            r#"
            WITH reachable AS (
                SELECT r.* FROM pg_catalog.pg_roles r
                WHERE pg_catalog.pg_has_role($1::text, r.oid, 'MEMBER')
            ), ca_schema AS (
                SELECT n.* FROM pg_catalog.pg_namespace n
                WHERE n.nspname = 'qadra_ca'
            ), ca_tables AS (
                SELECT c.* FROM pg_catalog.pg_class c
                JOIN ca_schema n ON n.oid = c.relnamespace
                WHERE c.relname IN ('users', 'audit_events')
            ), ca_columns AS (
                SELECT a.* FROM pg_catalog.pg_attribute a
                JOIN ca_tables c ON c.oid = a.attrelid
                WHERE a.attnum > 0 AND NOT a.attisdropped
            ), object_acl AS (
                SELECT a.grantee, a.is_grantable
                FROM ca_schema n
                CROSS JOIN LATERAL pg_catalog.aclexplode(
                    coalesce(n.nspacl, pg_catalog.acldefault('n', n.nspowner))
                ) a
                UNION ALL
                SELECT a.grantee, a.is_grantable
                FROM ca_tables c
                CROSS JOIN LATERAL pg_catalog.aclexplode(
                    coalesce(c.relacl, pg_catalog.acldefault('r', c.relowner))
                ) a
                UNION ALL
                SELECT a.grantee, a.is_grantable
                FROM ca_columns c
                CROSS JOIN LATERAL pg_catalog.aclexplode(c.attacl) a
            )
            SELECT
                EXISTS (
                    SELECT 1 FROM pg_catalog.pg_roles r
                    WHERE r.rolname = $1::text AND r.rolcanlogin
                )
                AND pg_catalog.has_schema_privilege(
                    $1::text, 'qadra_ca', 'USAGE'
                )
                AND pg_catalog.has_table_privilege(
                    $1::text, 'qadra_ca.users', 'SELECT'
                )
                AND pg_catalog.has_table_privilege(
                    $1::text, 'qadra_ca.audit_events', 'SELECT'
                )
                AND pg_catalog.has_table_privilege(
                    $1::text, 'qadra_ca.audit_events', 'INSERT'
                )
                AND pg_catalog.has_column_privilege(
                    $1::text, 'qadra_ca.users', 'recovery_codes', 'UPDATE'
                )
                AND pg_catalog.has_column_privilege(
                    $1::text, 'qadra_ca.users', 'revision', 'UPDATE'
                )
                AND pg_catalog.has_column_privilege(
                    $1::text, 'qadra_ca.users', 'updated_at', 'UPDATE'
                ) AS required,
                EXISTS (
                    SELECT 1 FROM reachable r
                    WHERE r.rolsuper OR r.rolcreaterole OR r.rolcreatedb
                        OR r.rolreplication OR r.rolbypassrls
                        OR pg_catalog.has_database_privilege(
                            r.oid, pg_catalog.current_database(), 'CREATE'
                        )
                        OR pg_catalog.has_schema_privilege(
                            r.oid, 'qadra_ca', 'CREATE'
                        )
                        OR EXISTS (
                            SELECT 1 FROM pg_catalog.pg_namespace n
                            WHERE n.nspname = 'public'
                                AND pg_catalog.has_schema_privilege(
                                    r.oid, n.oid, 'USAGE,CREATE'
                                )
                        )
                )
                OR EXISTS (
                    SELECT 1 FROM ca_schema n
                    WHERE pg_catalog.pg_has_role($1::text, n.nspowner, 'MEMBER')
                )
                OR EXISTS (
                    SELECT 1 FROM ca_tables c
                    WHERE pg_catalog.pg_has_role($1::text, c.relowner, 'MEMBER')
                )
                OR EXISTS (
                    SELECT 1 FROM reachable r
                    WHERE pg_catalog.has_table_privilege(
                        r.oid, 'qadra_ca.users',
                        'INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES'
                    ) OR pg_catalog.has_any_column_privilege(
                        r.oid, 'qadra_ca.users', 'INSERT,REFERENCES'
                    ) OR pg_catalog.has_table_privilege(
                        r.oid, 'qadra_ca.audit_events',
                        'UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES'
                    ) OR pg_catalog.has_any_column_privilege(
                        r.oid, 'qadra_ca.audit_events', 'UPDATE,REFERENCES'
                    ) OR EXISTS (
                        SELECT 1 FROM ca_columns a
                        JOIN ca_tables c ON c.oid = a.attrelid
                        WHERE c.relname = 'users'
                            AND a.attname NOT IN (
                                'recovery_codes', 'revision', 'updated_at'
                            )
                            AND pg_catalog.has_column_privilege(
                                r.oid, c.oid, a.attnum, 'UPDATE'
                            )
                    )
                ) AS excessive,
                EXISTS (
                    SELECT 1 FROM object_acl a
                    WHERE a.grantee = 0 OR (
                        a.is_grantable AND EXISTS (
                            SELECT 1 FROM reachable r WHERE r.oid = a.grantee
                        )
                    )
                ) AS delegated
            "#,
            &[&role],
        )
        .map_err(port)?;
    let required: bool = permissions.get(0);
    let excessive: bool = permissions.get(1);
    let delegated: bool = permissions.get(2);
    if !required || excessive || delegated {
        return Err(invalid(
            "CA identity runtime authority is unsafe or incomplete",
        ));
    }
    Ok(())
}
