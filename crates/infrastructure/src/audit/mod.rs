//! Audit log adapters.
//!
//! Two implementations of the domain's `AuditLog` port: a Vec-backed log for
//! tests and short-lived processes, and a JSON-lines file log that persists
//! the trail across process runs. Both compute chain values with the chain
//! rule defined in the domain, through any `DocumentHasher` implementation.

pub(crate) mod context;
mod file;
mod memory;

pub use file::{decode_audit_snapshot, FileAuditLog};
pub use memory::InMemoryAuditLog;
