//! Authorized, bounded consultation of existing audit events.

mod cursor;
mod model;
mod query;
mod service;

pub use model::*;
pub use query::AuditEventQuery;
pub use service::AuditEventService;

use crate::{identity::Principal, ApplicationError};
use domain::clock::OffsetDateTime;

pub trait AuditEventStore: Send + Sync {
    fn read(
        &self,
        actor: &Principal,
        query: &AuditEventQuery,
        at: OffsetDateTime,
    ) -> Result<AuditEventBatch, ApplicationError>;
}

pub trait AuditEventWorkflow: Send + Sync {
    fn read(&self, token: &str, query: AuditEventQuery)
        -> Result<AuditEventPage, ApplicationError>;
}
