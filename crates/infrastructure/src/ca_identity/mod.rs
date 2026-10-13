//! Separate PostgreSQL credential and audit boundary for the CA operator.

use application::ApplicationError;
use postgres::Client;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use zeroize::Zeroizing;

use crate::postgres_source::StartupConnection;

mod catalog;
mod constraints;
mod permissions;
mod provisioning;

pub use provisioning::provision_initial_ca_operator;

/// Startup-only authority for the fixed CA schema, never the office schema.
///
/// ```compile_fail
/// let escaped = infrastructure::with_validated_ca_identity_postgres(
///     "unused", "qadra", "qadra_ca_runtime", |source| {
///         Ok::<_, application::ApplicationError>(source)
///     },
/// );
/// ```
pub struct ValidatedCaIdentityPostgres<'scope> {
    url: Zeroizing<String>,
    database: String,
    role: String,
    identity: Vec<String>,
    validation: RefCell<StartupConnection>,
    scope: PhantomData<(&'scope (), Rc<()>)>,
}

/// Validates only CA credentials and audit storage before constructing adapters.
///
/// The callback constructs adapters without running application operations. The
/// existing schema and audit startup locks are released before those operations.
pub fn with_validated_ca_identity_postgres<T, E>(
    database_url: &str,
    expected_database: &str,
    expected_runtime_role: &str,
    compose: impl for<'scope> FnOnce(&ValidatedCaIdentityPostgres<'scope>) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<ApplicationError>,
{
    let mut client = crate::postgres::connect_runtime(database_url)?;
    catalog::target(&mut client, expected_database, expected_runtime_role)?;
    let mut validation = StartupConnection::from_client(client)?;
    let identity = validate(
        &mut validation.client,
        expected_database,
        expected_runtime_role,
    )?;
    let source = ValidatedCaIdentityPostgres {
        url: Zeroizing::new(database_url.to_owned()),
        database: expected_database.to_owned(),
        role: expected_runtime_role.to_owned(),
        identity,
        validation: RefCell::new(validation),
        scope: PhantomData,
    };
    compose(&source)
}

impl ValidatedCaIdentityPostgres<'_> {
    pub(crate) fn open_connection(&self) -> Result<Client, ApplicationError> {
        self.validation
            .borrow_mut()
            .client
            .simple_query("SELECT 1")
            .map_err(port)?;
        let mut client = crate::postgres::connect_runtime(&self.url)?;
        if validate(&mut client, &self.database, &self.role)? != self.identity {
            return Err(invalid(
                "CA identity PostgreSQL target changed during startup",
            ));
        }
        Ok(client)
    }

    pub(crate) fn connection_url(&self) -> &str {
        &self.url
    }
}

fn validate(
    client: &mut Client,
    database: &str,
    role: &str,
) -> Result<Vec<String>, ApplicationError> {
    let mut identity = catalog::validate(client, database, role)?;
    permissions::validate(client, role)?;
    identity.extend(crate::postgres_source::identity(client)?);
    Ok(identity)
}

fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidConfiguration(message.to_owned())
}

fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("CA identity PostgreSQL", error)
}
