//! Value objects shared by the cryptographic operations of the domain.
//!
//! Outbound ports (hashing, encryption, signing) are added alongside their
//! adapters. This module currently holds the immutable values those
//! operations exchange: content digests and document identity.

pub mod digest;
pub mod document;

pub use digest::Sha256Digest;
pub use document::{DocumentId, DocumentVersion};
