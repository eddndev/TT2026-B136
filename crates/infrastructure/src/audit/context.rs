//! Bounded storage encoding and explicit provenance for audit adapters.

use domain::audit::{AuditContext, AuditSource};
use domain::DomainError;

pub(crate) const MAX_CONTEXT_BYTES: usize = 1024;

pub(crate) fn current(default_source: AuditSource) -> Result<AuditContext, DomainError> {
    match application::audit_context::current() {
        Some(scope) => scope.snapshot().map_err(|_| invalid()),
        None => Ok(AuditContext {
            user_id: None,
            ip_address: None,
            source: default_source,
            request_id: None,
        }),
    }
}

pub(crate) fn encode(context: Option<&AuditContext>) -> Result<Option<String>, DomainError> {
    context
        .map(|value| {
            let encoded = serde_json::to_string(value).map_err(|_| invalid())?;
            if encoded.len() > MAX_CONTEXT_BYTES {
                return Err(invalid());
            }
            Ok(encoded)
        })
        .transpose()
}

pub(crate) fn decode(value: Option<&str>) -> Result<Option<AuditContext>, DomainError> {
    value
        .map(|encoded| {
            if encoded.len() > MAX_CONTEXT_BYTES {
                return Err(invalid());
            }
            serde_json::from_str(encoded).map_err(|_| invalid())
        })
        .transpose()
}

fn invalid() -> DomainError {
    DomainError::AuditStorageFailure("audit context is invalid or unavailable".into())
}
