//! Explicit scoped startup validation with no process-wide connection cache.
use application::ApplicationError;
use postgres::Client;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use zeroize::Zeroizing;

/// Accepted runtime connection sources; implementations are sealed.
///
/// ```compile_fail
/// struct Forged;
/// impl infrastructure::PostgresConnectionSource for Forged {}
/// ```
pub trait PostgresConnectionSource: sealed::Source {}
impl<T: sealed::Source + ?Sized> PostgresConnectionSource for T {}

/// A startup-only capability bound to one validated connection target.
///
/// It cannot be cloned, sent to a worker, constructed or returned by callers.
///
/// ```compile_fail
/// let escaped = infrastructure::with_validated_postgres("unused", |source| {
///     Ok::<_, application::ApplicationError>(source)
/// });
/// ```
pub struct ValidatedPostgres<'scope> {
    url: Zeroizing<String>,
    identity: Vec<String>,
    validation: RefCell<StartupConnection>,
    scope: PhantomData<(&'scope (), Rc<()>)>,
}

/// Validates all runtime families before constructing independently owned adapters.
///
/// The callback only constructs adapters. Their database operations begin after
/// it returns, when the validation connection releases its audit mutation lock.
pub fn with_validated_postgres<T, E>(
    database_url: &str,
    compose: impl for<'scope> FnOnce(&ValidatedPostgres<'scope>) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<ApplicationError>,
{
    let mut validation = StartupConnection::new(database_url)?;
    crate::postgres::validate_runtime(&mut validation.client)?;
    let context = ValidatedPostgres {
        url: Zeroizing::new(database_url.to_owned()),
        identity: identity(&mut validation.client)?,
        validation: RefCell::new(validation),
        scope: PhantomData,
    };
    compose(&context)
}

struct StartupConnection {
    client: Client,
    schema: i32,
    audited: bool,
}
impl StartupConnection {
    fn new(url: &str) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::connect_runtime(url)?;
        // Match migration ordering before excluding all audited mutations.
        let schema = crate::postgres::lock_startup_schema(&mut client)?;
        let mut guard = Self {
            client,
            schema,
            audited: false,
        };
        crate::audit_postgres::lock_startup(&mut guard.client)?;
        guard.audited = true;
        Ok(guard)
    }
}
impl Drop for StartupConnection {
    fn drop(&mut self) {
        // Explicit release completes before returning adapters to their consumers.
        // A lost connection releases all session locks when PostgreSQL closes it.
        if self.audited {
            let _ = crate::audit_postgres::unlock_startup(&mut self.client);
        }
        let _ = crate::postgres::unlock_startup_schema(&mut self.client, self.schema);
    }
}

pub(crate) fn connect(
    source: &(impl PostgresConnectionSource + ?Sized),
) -> Result<Client, ApplicationError> {
    sealed::Source::connect(source)
}
pub(crate) fn url(source: &(impl PostgresConnectionSource + ?Sized)) -> &str {
    sealed::Source::url(source)
}

mod sealed {
    use super::*;

    pub trait Source {
        fn connect(&self) -> Result<Client, ApplicationError>;
        fn url(&self) -> &str;
    }
    impl Source for str {
        fn connect(&self) -> Result<Client, ApplicationError> {
            crate::postgres::open_url(self)
        }
        fn url(&self) -> &str {
            self
        }
    }
    impl Source for String {
        fn connect(&self) -> Result<Client, ApplicationError> {
            self.as_str().connect()
        }
        fn url(&self) -> &str {
            self.as_str()
        }
    }
    impl Source for Zeroizing<String> {
        fn connect(&self) -> Result<Client, ApplicationError> {
            self.as_str().connect()
        }
        fn url(&self) -> &str {
            self.as_str()
        }
    }
    impl<T: Source + ?Sized> Source for &T {
        fn connect(&self) -> Result<Client, ApplicationError> {
            (**self).connect()
        }
        fn url(&self) -> &str {
            (**self).url()
        }
    }
    impl Source for ValidatedPostgres<'_> {
        fn connect(&self) -> Result<Client, ApplicationError> {
            let mut client = crate::postgres::connect_runtime(&self.url)?;
            if identity(&mut client)? != self.identity {
                return Err(ApplicationError::InvalidConfiguration(
                    "startup PostgreSQL connection target changed".into(),
                ));
            }
            self.validation
                .borrow_mut()
                .client
                .simple_query("SELECT 1")
                .map_err(crate::postgres_connection::port_error)?;
            Ok(client)
        }
        fn url(&self) -> &str {
            &self.url
        }
    }
}

fn identity(client: &mut Client) -> Result<Vec<String>, ApplicationError> {
    client
        .query_one(
            "SELECT ARRAY[current_database(),
             (SELECT oid::text FROM pg_catalog.pg_database WHERE datname=current_database()),
             current_user::text,session_user::text,
             current_user::regrole::oid::text,session_user::regrole::oid::text,
             COALESCE(current_schema(),''),current_setting('search_path'),
             COALESCE(inet_server_addr()::text,'local'),
             COALESCE(inet_server_port()::text,'local'),
             extract(epoch FROM pg_postmaster_start_time())::text]",
            &[],
        )
        .map(|row| row.get(0))
        .map_err(crate::postgres_connection::port_error)
}
