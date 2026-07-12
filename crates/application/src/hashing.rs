//! Use case: compute the SHA-256 digest of a document stream.

use std::io::Read;

use domain::crypto::{DocumentHasher, Sha256Digest};

use crate::error::ApplicationError;

/// Digests a document supplied as a stream through a hashing port.
pub struct HashDocument<H> {
    hasher: H,
}

impl<H: DocumentHasher> HashDocument<H> {
    /// Builds the use case over any [`DocumentHasher`] implementation.
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }

    /// Consumes `reader` to the end and returns the digest of its content.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::Domain`] when reading the stream fails.
    pub fn execute(&self, reader: &mut dyn Read) -> Result<Sha256Digest, ApplicationError> {
        Ok(self.hasher.hash_stream(reader)?)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::crypto::{DocumentHasher, Sha256Digest};
    use domain::DomainError;
    use mockall::mock;

    use super::HashDocument;
    use crate::error::ApplicationError;

    mock! {
        Hasher {}

        impl DocumentHasher for Hasher {
            fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
            fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
        }
    }

    #[test]
    fn streams_the_reader_through_the_port_and_returns_its_digest() {
        let digest = Sha256Digest::from_array([7u8; 32]);
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash_stream()
            .times(1)
            .returning(move |reader| {
                let mut consumed = Vec::new();
                reader.read_to_end(&mut consumed).unwrap();
                assert_eq!(consumed, b"case file body");
                Ok(digest)
            });

        let use_case = HashDocument::new(hasher);
        let result = use_case.execute(&mut &b"case file body"[..]).unwrap();
        assert_eq!(result, digest);
    }

    #[test]
    fn surfaces_a_stream_read_failure_as_a_domain_error() {
        let mut hasher = MockHasher::new();
        hasher.expect_hash_stream().times(1).returning(|_| {
            Err(DomainError::StreamRead {
                message: "broken pipe".to_string(),
            })
        });

        let use_case = HashDocument::new(hasher);
        let err = use_case.execute(&mut std::io::empty()).unwrap_err();
        match err {
            ApplicationError::Domain(DomainError::StreamRead { message }) => {
                assert_eq!(message, "broken pipe");
            }
            other => panic!("expected a stream read error, got: {other}"),
        }
    }
}
