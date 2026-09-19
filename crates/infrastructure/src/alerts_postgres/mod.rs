//! Personal alert persistence and durable, audited scheduling and delivery.
mod authorization;
mod codec;
mod delivery;
mod inbox;
mod invalidate;
mod inventory;
mod plan;
mod preferences;
mod records;
mod schedule_activate;
mod schedule_rows;
mod schedule_scan;
mod schedule_selection;
mod scheduler;
mod subject;

use application::{alerts::*, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
pub(crate) use invalidate::deadline as invalidate_deadline;
pub(crate) use invalidate::invalidate;
pub(crate) use inventory::validate as validate_inventory;
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use time::{OffsetDateTime, UtcOffset};

pub struct PostgresAlertStore {
    client: Mutex<Client>,
    url: String,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    email: Option<AlertEmailConfiguration>,
}

impl PostgresAlertStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
        email: Option<AlertEmailConfiguration>,
    ) -> Result<Self, ApplicationError> {
        if let Some(config) = &email {
            crate::alert_email::validate_configuration(config)?;
        }
        Ok(Self {
            client: Mutex::new(connect(url)?),
            url: url.into(),
            hasher,
            clock,
            email,
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| stored("alert connection lock poisoned"))?;
        if client.is_closed() {
            *client = connect(&self.url)?;
        }
        Ok(client)
    }

    fn now(&self) -> Result<OffsetDateTime, ApplicationError> {
        let now = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&now.year()) {
            return Err(stored("alert clock outside supported years"));
        }
        Ok(now)
    }

    fn transport(&self) -> AlertEmailTransport {
        if self.email.is_some() {
            AlertEmailTransport::Ready
        } else {
            AlertEmailTransport::Disabled
        }
    }
}

fn connect(url: &str) -> Result<Client, ApplicationError> {
    let mut client = crate::postgres::open(url)?;
    client
        .batch_execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
        .map_err(port)?;
    Ok(client)
}
fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("alert database", error)
}
fn stored(error: impl std::fmt::Display) -> ApplicationError {
    AlertError::Stored(error.to_string()).into()
}
fn audit(
    tx: &mut postgres::Transaction<'_>,
    actor: &str,
    action: &str,
    resource: &str,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    crate::audit_postgres::append_transaction(tx, actor, action, resource, at).map(|_| ())
}

impl AlertStore for PostgresAlertStore {
    fn preferences(
        &self,
        actor: domain::identity::UserId,
    ) -> Result<AlertPreferences, ApplicationError> {
        preferences::get(self, actor)
    }
    fn save_preferences(
        &self,
        actor: domain::identity::UserId,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError> {
        preferences::save(self, actor, command)
    }
    fn list(
        &self,
        actor: domain::identity::UserId,
        query: AlertQuery,
    ) -> Result<AlertPage, ApplicationError> {
        inbox::list(self, actor, query)
    }
    fn get(
        &self,
        actor: domain::identity::UserId,
        id: AlertId,
    ) -> Result<AlertDetail, ApplicationError> {
        inbox::get(self, actor, id)
    }
    fn mark_read(
        &self,
        actor: domain::identity::UserId,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError> {
        inbox::mark_read(self, actor, command)
    }
}
