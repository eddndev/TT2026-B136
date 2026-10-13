//! Administrative insertion of the separately prepared initial CA operator.

use application::{identity::UserRecord, ApplicationError};
use domain::clock::OffsetDateTime;
use domain::identity::Role;

use crate::{
    audit_postgres::{append_transaction, begin_audited},
    identity::postgres::insert_user,
    postgres_source::StartupConnection,
};

use super::{catalog, invalid, port};

/// Persists the first CA account and its audit event in one transaction.
///
/// The caller prepares the credential with `IdentityService::prepare_ca_operator`
/// and confirms its second factor before calling this administrative boundary.
/// Only the exact configured database owner may provision the existing CA schema;
/// this function never creates a schema, changes grants, or replaces an account.
pub fn provision_initial_ca_operator(
    admin_url: &str,
    expected_database: &str,
    expected_admin_role: &str,
    user: UserRecord,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if user.role != Role::Client || !user.active {
        return Err(ApplicationError::PermissionDenied);
    }
    if user.id.as_uuid().is_nil() || user.revision != 0 || user.auth_generation != 0 {
        return Err(ApplicationError::InvalidInput(
            "CA enrollment requires a new operator record".into(),
        ));
    }
    if expected_database.is_empty() || expected_admin_role.is_empty() {
        return Err(invalid(
            "CA provisioning requires an explicit database and owner",
        ));
    }
    let mut client = crate::postgres::connect_runtime(admin_url)?;
    catalog::target(&mut client, expected_database, expected_admin_role)?;
    let mut startup = StartupConnection::from_client(client)?;
    let mut transaction = begin_audited(&mut startup.client)?;
    let owner: bool = transaction
        .query_one(
            "SELECT d.datdba = r.oid FROM pg_catalog.pg_database d
         JOIN pg_catalog.pg_roles r ON r.rolname=current_user
         WHERE d.datname=pg_catalog.current_database()",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !owner {
        return Err(ApplicationError::PermissionDenied);
    }
    transaction
        .batch_execute(
            "LOCK TABLE qadra_ca.users, qadra_ca.audit_events IN SHARE ROW EXCLUSIVE MODE",
        )
        .map_err(port)?;
    catalog::validate(&mut transaction, expected_database, expected_admin_role)?;
    if transaction
        .query_one("SELECT EXISTS(SELECT 1 FROM qadra_ca.users)", &[])
        .map_err(port)?
        .get::<_, bool>(0)
    {
        return Err(ApplicationError::UserAlreadyExists);
    }
    insert_user(&mut transaction, &user)?;
    append_transaction(
        &mut transaction,
        expected_admin_role,
        "ca.operator_enrolled",
        &user.id.to_string(),
        at,
    )?;
    transaction.commit().map_err(port)
}
