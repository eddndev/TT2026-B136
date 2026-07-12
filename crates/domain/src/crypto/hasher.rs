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

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::DocumentHasher;
    use crate::crypto::digest::{Sha256Digest, SHA256_LEN};
    use crate::error::DomainError;

    /// Test double: "hashes" by summing bytes into the first digest byte.
    ///
    /// It is not a real hash; it only exercises the port signatures.
    struct SummingHasher;

    impl DocumentHasher for SummingHasher {
        fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
            let mut bytes = [0u8; SHA256_LEN];
            bytes[0] = data.iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
            Sha256Digest::from_array(bytes)
        }

        fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
            let mut data = Vec::new();
            reader
                .read_to_end(&mut data)
                .map_err(|err| DomainError::StreamRead {
                    message: err.to_string(),
                })?;
            Ok(self.hash_bytes(&data))
        }
    }

    /// A reader whose first read always fails.
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("simulated stream failure"))
        }
    }

    #[test]
    fn port_is_object_safe() {
        let hasher: &dyn DocumentHasher = &SummingHasher;
        let digest = hasher.hash_bytes(&[1, 2, 3]);
        assert_eq!(digest.as_bytes()[0], 6);
    }

    #[test]
    fn hash_stream_reports_matching_digest_for_same_input() {
        let hasher = SummingHasher;
        let data = [10u8, 20, 30];
        let from_slice = hasher.hash_bytes(&data);
        let from_stream = hasher.hash_stream(&mut &data[..]).unwrap();
        assert_eq!(from_slice, from_stream);
    }

    #[test]
    fn hash_stream_surfaces_read_failures() {
        let hasher = SummingHasher;
        let err = hasher.hash_stream(&mut FailingReader).unwrap_err();
        assert_eq!(
            err,
            DomainError::StreamRead {
                message: "simulated stream failure".to_string(),
            }
        );
    }

    #[test]
    fn stream_read_error_displays_its_message() {
        let err = DomainError::StreamRead {
            message: "disk detached".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "failed to read input stream: disk detached"
        );
    }
}
