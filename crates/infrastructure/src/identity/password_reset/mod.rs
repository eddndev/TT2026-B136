//! Internal PostgreSQL password recovery using restricted audited operations.

mod repository;
mod values;

use std::sync::{Mutex, MutexGuard};

use application::ApplicationError;
use postgres::Client;
use values::{invalid_state, storage_error};

/// Durable capabilities and password changes without an HTTP or email adapter.
pub struct PostgresPasswordResetRepository {
    client: Mutex<Client>,
    schema: String,
}

impl PostgresPasswordResetRepository {
    /// Administrative connection that applies the normal idempotent migrations.
    pub fn connect(database_url: &str) -> Result<Self, ApplicationError> {
        let client = crate::postgres::connect(database_url).map_err(|_| storage_error())?;
        Self::from_client(client)
    }

    /// Opens the fully validated schema using the restricted runtime role.
    pub fn open(
        database_url: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        let client = crate::postgres::open(database_url).map_err(|_| storage_error())?;
        Self::from_client(client)
    }

    fn from_client(mut client: Client) -> Result<Self, ApplicationError> {
        let schema = client
            .query_one(
                "SELECT pg_catalog.quote_ident(n.nspname)
                 FROM pg_catalog.pg_class c
                 JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
                 WHERE c.oid='password_reset_capabilities'::pg_catalog.regclass",
                &[],
            )
            .map_err(|_| storage_error())?
            .try_get(0)
            .map_err(|_| invalid_state())?;
        Ok(Self {
            client: Mutex::new(client),
            schema,
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| storage_error())
    }
}
