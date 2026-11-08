//! Outbound port for computing SHA-256 content digests.

use std::io::Read;

use crate::crypto::digest::Sha256Digest;
use crate::error::DomainError;

/// Computes SHA-256 digests of document content.
///
/// Adapters implement this port with a concrete cryptographic backend. Both
/// operations must produce the same digest for the same input bytes.
pub trait DocumentHasher {
    /// Hashes a byte slice already held in memory.
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;

    /// Hashes a stream by consuming it in fixed-size blocks, so inputs of
    /// any length are digested without loading them fully into memory.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::StreamRead`] when reading from `reader` fails.
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
}
