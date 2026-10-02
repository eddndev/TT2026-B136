//! Bounded activity pages read and recorded in one authorization transaction.

mod selection;

use application::{audit_query::*, identity::Principal, ApplicationError};
use domain::identity::Role;
use postgres::Client;
use std::sync::Mutex;
use time::OffsetDateTime;

pub struct PostgresAuditEventStore {
    client: Mutex<Client>,
}

impl PostgresAuditEventStore {
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(source)?;
        client
            .batch_execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}

impl AuditEventStore for PostgresAuditEventStore {
    fn read(
        &self,
        actor: &Principal,
        query: &AuditEventQuery,
        at: OffsetDateTime,
    ) -> Result<AuditEventBatch, ApplicationError> {
        if actor.role != Role::Owner {
            return Err(ApplicationError::PermissionDenied);
        }
        let mut client = self
            .client
            .lock()
            .map_err(|_| invalid("database lock poisoned"))?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let active = crate::postgres_actor::active_actor(&mut tx, actor.id)?;
        if active != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        let current: Option<i64> = tx
            .query_one("SELECT max(sequence) FROM audit_events", &[])
            .map_err(port)?
            .get(0);
        let current = current
            .map(u64::try_from)
            .transpose()
            .map_err(|_| invalid("negative head"))?;
        let snapshot = match query.cursor() {
            Some(cursor) if current.is_some_and(|head| cursor.snapshot_max_sequence <= head) => {
                Some(cursor.snapshot_max_sequence)
            }
            Some(_) => {
                return Err(ApplicationError::InvalidInput(
                    "audit cursor exceeds the current head".into(),
                ))
            }
            None => current,
        };
        let page = match snapshot {
            Some(snapshot) => selection::read(&mut tx, query, snapshot)?,
            None => AuditEventBatch {
                snapshot_max_sequence: None,
                events: Vec::new(),
                has_more: false,
            },
        };
        crate::audit_postgres::append_transaction(
            &mut tx,
            &active.email,
            "audit.events_read",
            &format!(
                "audit:through:{}:returned:{}",
                snapshot.map_or_else(|| "none".into(), |s| s.to_string()),
                page.events.len()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(page)
    }
}

fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("audit query database", error)
}
fn invalid(message: &str) -> ApplicationError {
    ApplicationError::Port(format!("invalid audit query result: {message}"))
}
