//! Grants and role memberships that give the runtime role excess authority.

use super::ModuleSpec;

/// Direct grants, PUBLIC, grant options and authority reachable through
/// membership: inherited, NOINHERIT and SET ROLE only.
pub(super) fn privilege_mutations(spec: &ModuleSpec) -> Vec<(String, String)> {
    let create_helper = "CREATE ROLE {helper} NOLOGIN;";
    let mut out = Vec::new();
    for table in spec.tables {
        for (kind, sql) in [
            ("delete granted", format!("GRANT DELETE ON {table} TO {{runtime}}")),
            ("truncate granted", format!("GRANT TRUNCATE ON {table} TO {{runtime}}")),
            ("select revoked", format!("REVOKE SELECT ON {table} FROM {{runtime}}")),
            ("public select", format!("GRANT SELECT ON {table} TO PUBLIC")),
            (
                "grant option",
                format!("GRANT SELECT ON {table} TO {{runtime}} WITH GRANT OPTION"),
            ),
            (
                "inherited",
                format!("{create_helper} GRANT TRUNCATE ON {table} TO {{helper}}; GRANT {{helper}} TO {{runtime}}"),
            ),
            (
                "noinherit role",
                format!(
                    "{create_helper} GRANT TRUNCATE ON {table} TO {{helper}};
                    ALTER ROLE {{runtime}} NOINHERIT; GRANT {{helper}} TO {{runtime}}"
                ),
            ),
            (
                "set role only",
                format!(
                    "{create_helper} GRANT TRUNCATE ON {table} TO {{helper}};
                    GRANT {{helper}} TO {{runtime}} WITH INHERIT FALSE, SET TRUE"
                ),
            ),
            (
                "owner by set role",
                format!(
                    "{create_helper} ALTER TABLE {table} OWNER TO {{helper}};
                    GRANT {{helper}} TO {{runtime}} WITH INHERIT FALSE, SET TRUE"
                ),
            ),
        ] {
            out.push((format!("{table} {kind}"), sql));
        }
    }
    for (table, column) in spec.protected_columns {
        let name = format!("{table}.{column}");
        out.push((
            format!("{name} update granted"),
            format!("GRANT UPDATE({column}) ON {table} TO {{runtime}}"),
        ));
        out.push((
            format!("{name} public update"),
            format!("GRANT UPDATE({column}) ON {table} TO PUBLIC"),
        ));
    }
    for signature in spec.private_functions {
        out.push((
            format!("{signature} execute granted"),
            format!("GRANT EXECUTE ON FUNCTION {signature} TO {{runtime}}"),
        ));
        out.push((
            format!("{signature} public execute"),
            format!("GRANT EXECUTE ON FUNCTION {signature} TO PUBLIC"),
        ));
    }
    out.push((
        "role administration through membership".into(),
        "CREATE ROLE {helper} NOLOGIN CREATEROLE; GRANT {helper} TO {runtime} WITH INHERIT FALSE"
            .to_owned(),
    ));
    out
}
