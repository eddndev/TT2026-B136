//! Durable outcomes of admitted HTTP operations.

use crate::{audit_context::AuditScope, ApplicationError};
use domain::{
    audit::{AuditContext, AuditLog},
    clock::Clock,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AccessAuditRecord {
    pub context: AuditContext,
    pub action: String,
    pub resource: String,
}

/// Returning success means that the append has reached durable storage.
pub trait AccessAuditWriter: Send + Sync {
    fn record(&self, record: AccessAuditRecord) -> Result<(), ApplicationError>;
}

pub struct AccessAuditService {
    log: Mutex<Box<dyn AuditLog + Send>>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl AccessAuditService {
    pub fn new(log: Box<dyn AuditLog + Send>, clock: Arc<dyn Clock + Send + Sync>) -> Self {
        Self {
            log: Mutex::new(log),
            clock,
        }
    }
}

impl AccessAuditWriter for AccessAuditService {
    fn record(&self, record: AccessAuditRecord) -> Result<(), ApplicationError> {
        let actor = record
            .context
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unavailable".into());
        let scope = AuditScope::new(record.context);
        scope.run(|| {
            self.log
                .lock()
                .map_err(|_| ApplicationError::Port("audit writer lock poisoned".into()))?
                .append(&actor, &record.action, &record.resource, self.clock.now())?;
            Ok(())
        })
    }
}
