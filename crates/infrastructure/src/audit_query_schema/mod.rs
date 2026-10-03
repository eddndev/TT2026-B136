//! Exact additive audit query projection and immutable-history permissions.
mod catalog;
mod functions;
mod permissions;
mod references;

use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};
pub(crate) use references::validate as validate_reference_triggers;
pub(crate) const MIGRATION: &str =
    include_str!("../../../../migrations/0027_audit_query_projection.sql");

pub(crate) fn validate_inventory<C: postgres::GenericClient>(
    client: &mut C,
) -> Result<(), ApplicationError> {
    let bad: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events
        WHERE timestamp_seconds IS DISTINCT FROM (audit_timestamp_parts(timestamp))[1]
        OR timestamp_nanos::bigint IS DISTINCT FROM (audit_timestamp_parts(timestamp))[2])",
            &[],
        )
        .map_err(port)?
        .get(0);
    if bad {
        return Err(incomplete());
    }
    Ok(())
}
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("audit query projection is incomplete or altered".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("audit query schema", error)
}
