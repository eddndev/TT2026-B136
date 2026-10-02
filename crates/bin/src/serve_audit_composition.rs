//! Compose the bounded audit reader after PostgreSQL schema validation.
use anyhow::Context;
use application::{
    audit_query::{AuditEventService, AuditEventWorkflow},
    identity::IdentityWorkflow,
};
use infrastructure::{PostgresAuditEventStore, PostgresConnectionSource, SystemClock};
use std::sync::Arc;

pub(crate) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<Arc<dyn AuditEventWorkflow>> {
    let store = PostgresAuditEventStore::open(database)
        .context("cannot open PostgreSQL audit event store")?;
    Ok(Arc::new(AuditEventService::new(
        Arc::new(store),
        identity,
        Arc::new(SystemClock::new()),
    )))
}
