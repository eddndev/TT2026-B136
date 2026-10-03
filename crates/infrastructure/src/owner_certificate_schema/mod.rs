//! Exact catalog and append-only authority for global Owner certificate history.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod permissions;
mod specifications;
mod triggers;

use application::ApplicationError;
pub(crate) use catalog::validate_schema;
pub(crate) use inventory::validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime};

pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0029_owner_certificate_tables.sql"),
    include_str!("../../../../migrations/0029_owner_certificate_registration.sql"),
    include_str!("../../../../migrations/0029_owner_certificate_withdrawal.sql"),
];
const TABLES: [&str; 2] = [
    "owner_certificate_registrations",
    "owner_certificate_withdrawals",
];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "owner certificate schema is incomplete or altered".into(),
    )
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("owner certificate schema operation failed".into())
}
