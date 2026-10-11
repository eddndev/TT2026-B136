//! Compose the bounded audit reader after PostgreSQL schema validation.
use anyhow::Context;
use application::{
    audit_query::{AuditEventService, AuditEventWorkflow},
    identity::IdentityWorkflow,
};
use infrastructure::{PostgresAuditEventStore, PostgresConnectionSource, SystemClock};
use std::sync::Arc;

pub(crate) fn protect(
    database: &(impl PostgresConnectionSource + ?Sized),
    router: axum::Router,
) -> anyhow::Result<(axum::Router, web::AccessAuditDrain)> {
    let proxy = match std::env::var("TT_AUDIT_TRUST_LOOPBACK_PROXY") {
        Ok(value) if value == "true" => web::AuditProxyPolicy::LoopbackProxy,
        Ok(value) if value == "false" => web::AuditProxyPolicy::Direct,
        Err(std::env::VarError::NotPresent) => web::AuditProxyPolicy::Direct,
        _ => anyhow::bail!("TT_AUDIT_TRUST_LOOPBACK_PROXY must be true or false"),
    };
    let log = infrastructure::PostgresAuditLog::open(database)
        .context("cannot open durable access audit writer")?;
    let writer = Arc::new(application::access_audit::AccessAuditService::new(
        Box::new(log),
        Arc::new(SystemClock::new()),
    ));
    let drain = web::AccessAuditDrain::default();
    Ok((
        web::with_access_audit_drain(router, writer, proxy, drain.clone()),
        drain,
    ))
}

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
