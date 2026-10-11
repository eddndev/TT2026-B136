//! Schema and least-privilege runtime access for recoverable invitations.
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) const MIGRATION: &str = include_str!("../../../migrations/0046_member_invitations.sql");
const TABLES: [&str; 3] = [
    "member_invitations",
    "member_invitation_operations",
    "member_invitation_completions",
];
const MUTABLE:&str="revision,operation_id,issued_at,expires_at,accepted_at,cancelled_at,token_digest,protected_token,pending_operation,pending_digest,pending_material,delivery_state,delivery_generation,lease_id,lease_until,provider_deadline,retry_at,audit_sequence";

pub(crate) fn grant_runtime<C: GenericClient>(
    c: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let quoted: String = c
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    for table in TABLES {
        c.batch_execute(&format!(
            "REVOKE ALL ON {table} FROM {quoted}; GRANT SELECT,INSERT ON {table} TO {quoted}"
        ))
        .map_err(port)?;
    }
    c.batch_execute(&format!(
        "GRANT UPDATE({MUTABLE}) ON member_invitations TO {quoted}"
    ))
    .map_err(port)
}
pub(crate) fn validate_schema<C: GenericClient>(c: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid:bool=c.query_one("SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_class u ON u.oid='users'::regclass WHERE c.oid=to_regclass($1) AND c.relnamespace=u.relnamespace AND c.relowner=u.relowner AND c.relkind='r' AND c.relpersistence='p' AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity AND NOT c.relispartition)",&[&table]).map_err(port)?.get(0);
        if !valid {
            return Err(invalid());
        }
    }
    let valid: bool = c
        .query_one(
            "SELECT NOT EXISTS(
                SELECT 1 FROM member_invitations
                WHERE role NOT IN ('litigator','paralegal') OR revision<1
                    OR expires_at<=issued_at OR octet_length(token_digest)<>32
                    OR NOT (
                        (pending_operation IS NULL AND pending_digest IS NULL
                            AND pending_material IS NULL)
                        OR (pending_operation IS NOT NULL AND pending_digest IS NOT NULL
                            AND pending_material IS NOT NULL AND octet_length(pending_digest)=32
                            AND octet_length(pending_material) BETWEEN 29 AND 65564))
                    OR NOT (
                        (accepted_at IS NULL AND cancelled_at IS NULL
                            AND protected_token IS NOT NULL
                            AND octet_length(protected_token) BETWEEN 29 AND 65564)
                        OR ((accepted_at IS NOT NULL OR cancelled_at IS NOT NULL)
                            AND protected_token IS NULL AND pending_material IS NULL))
                    OR (cancelled_at IS NOT NULL AND (accepted_at IS NOT NULL
                        OR cancelled_at<issued_at OR protected_token IS NOT NULL
                        OR pending_material IS NOT NULL OR lease_id IS NOT NULL
                        OR retry_at IS NOT NULL)))
                AND NOT EXISTS(
                    SELECT 1 FROM member_invitation_completions c
                    JOIN member_invitations i ON i.id=c.invitation_id
                    WHERE i.accepted_at IS NULL OR i.token_digest<>c.token_digest)",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn validate_runtime<C: GenericClient>(
    c: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    validate_schema(c)?;
    for table in TABLES {
        let unsafe_role:bool=c.query_one("SELECT NOT has_table_privilege($1,$2,'SELECT') OR NOT has_table_privilege($1,$2,'INSERT') OR has_table_privilege($1,$2,'UPDATE,DELETE,TRUNCATE,TRIGGER,REFERENCES') OR EXISTS(SELECT 1 FROM pg_class c, LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a WHERE c.oid=$2::text::regclass AND (a.grantee=0 OR (a.is_grantable AND a.grantee<>c.relowner)))",&[&role,&table]).map_err(port)?.get(0);
        if unsafe_role {
            return Err(invalid());
        }
        if table != "member_invitations" {
            let updates: bool = c
                .query_one(
                    "SELECT has_any_column_privilege($1,$2,'UPDATE')",
                    &[&role, &table],
                )
                .map_err(port)?
                .get(0);
            if updates {
                return Err(invalid());
            }
        }
    }
    for column in ["id", "email", "role"] {
        let allowed: bool = c
            .query_one(
                "SELECT has_column_privilege($1,'member_invitations',$2,'UPDATE')",
                &[&role, &column],
            )
            .map_err(port)?
            .get(0);
        if allowed {
            return Err(invalid());
        }
    }
    for column in MUTABLE.split(',') {
        let allowed: bool = c
            .query_one(
                "SELECT has_column_privilege($1,'member_invitations',$2,'UPDATE')",
                &[&role, &column],
            )
            .map_err(port)?
            .get(0);
        if !allowed {
            return Err(invalid());
        }
    }
    Ok(())
}
fn invalid() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "invitation schema or runtime permissions are incomplete".into(),
    )
}
fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("invitation schema operation failed".into())
}
