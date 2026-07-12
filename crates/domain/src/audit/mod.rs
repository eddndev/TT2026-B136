//! Append-only audit trail.
//!
//! The audit trail records actions taken on documents and cases. Each entry
//! is chained to the previous one with a SHA-256 hash so that altering an
//! entry, inserting one, or deleting or reordering entries anywhere before
//! the last one breaks verification of the trail. Truncating the tail is
//! not detected by verification alone; the limitation and its handling are
//! recorded in docs/adr/0007-audit-chain-anchoring.md. The chain rule and
//! the canonical byte encoding it hashes are documented where they are
//! implemented, in the `chain` and `event` submodules; hashing goes through
//! the [`crate::crypto::DocumentHasher`] port so no cryptographic backend
//! enters this crate.

pub mod chain;
pub mod event;
pub mod port;

pub use chain::{chain_digest, verify_chain, ChainVerification, GENESIS_PREVIOUS};
pub use event::{AuditEvent, ChainedEvent};
pub use port::AuditLog;
