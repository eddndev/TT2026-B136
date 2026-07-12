//! SHA-256 hashing adapter backed by the ring crate.

use std::io::{ErrorKind, Read};

use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use ring::digest::{Context, Digest, SHA256};

/// Bytes read per block while digesting a stream: 64 KiB keeps memory flat
/// no matter how large the input is.
const STREAM_BLOCK_LEN: usize = 64 * 1024;

/// [`DocumentHasher`] backed by ring's SHA-256 implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct RingSha256Hasher;

impl RingSha256Hasher {
    /// Creates the adapter; it holds no state.
    pub fn new() -> Self {
        Self
    }
}

impl DocumentHasher for RingSha256Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        into_domain_digest(ring::digest::digest(&SHA256, data))
    }

    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut context = Context::new(&SHA256);
        let mut block = vec![0u8; STREAM_BLOCK_LEN];
        loop {
            match reader.read(&mut block) {
                Ok(0) => break,
                Ok(filled) => context.update(&block[..filled]),
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => {
                    return Err(DomainError::StreamRead {
                        message: err.to_string(),
                    })
                }
            }
        }
        Ok(into_domain_digest(context.finish()))
    }
}

fn into_domain_digest(digest: Digest) -> Sha256Digest {
    Sha256Digest::from_bytes(digest.as_ref()).expect("ring SHA-256 always yields 32 bytes")
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use domain::crypto::DocumentHasher;
    use domain::DomainError;

    use super::RingSha256Hasher;

    /// Deterministic pseudo-random bytes (xorshift32), so tests need no
    /// random-number dependency and always reproduce.
    fn pseudo_random_bytes(len: usize) -> Vec<u8> {
        let mut state: u32 = 0x2026_b136;
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            out.extend_from_slice(&state.to_le_bytes());
        }
        out.truncate(len);
        out
    }

    /// Reader that serves its data in a repeating cycle of irregular chunk
    /// sizes, exercising the block loop of the streaming implementation.
    struct IrregularChunkReader {
        data: Vec<u8>,
        position: usize,
        step: usize,
    }

    impl IrregularChunkReader {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                position: 0,
                step: 0,
            }
        }
    }

    impl Read for IrregularChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            const CHUNK_SIZES: [usize; 7] = [1, 3, 7, 64, 1_000, 4_096, 65_537];
            let remaining = self.data.len() - self.position;
            if remaining == 0 {
                return Ok(0);
            }
            let wanted = CHUNK_SIZES[self.step % CHUNK_SIZES.len()];
            self.step += 1;
            let len = wanted.min(buf.len()).min(remaining);
            buf[..len].copy_from_slice(&self.data[self.position..self.position + len]);
            self.position += len;
            Ok(len)
        }
    }

    /// Reader whose first read is interrupted and second read fails hard.
    struct InterruptedThenFailingReader {
        calls: usize,
    }

    impl Read for InterruptedThenFailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls == 1 {
                Err(io::Error::new(io::ErrorKind::Interrupted, "interrupted"))
            } else {
                Err(io::Error::other("simulated stream failure"))
            }
        }
    }

    // Known-answer tests from the FIPS 180-4 example vectors for SHA-256.

    #[test]
    fn hashes_the_empty_input() {
        let digest = RingSha256Hasher::new().hash_bytes(b"");
        assert_eq!(
            digest.to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hashes_abc() {
        let digest = RingSha256Hasher::new().hash_bytes(b"abc");
        assert_eq!(
            digest.to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_the_two_block_message() {
        let message = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        let digest = RingSha256Hasher::new().hash_bytes(message);
        assert_eq!(
            digest.to_hex(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn hashes_one_million_a_bytes_streamed() {
        let mut reader = io::repeat(b'a').take(1_000_000);
        let digest = RingSha256Hasher::new().hash_stream(&mut reader).unwrap();
        assert_eq!(
            digest.to_hex(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn slice_and_irregular_stream_agree() {
        let data = pseudo_random_bytes(1_000_003);
        let hasher = RingSha256Hasher::new();
        let from_slice = hasher.hash_bytes(&data);
        let mut reader = IrregularChunkReader::new(data);
        let from_stream = hasher.hash_stream(&mut reader).unwrap();
        assert_eq!(from_slice, from_stream);
    }

    #[test]
    fn stream_failure_maps_to_stream_read_after_retrying_interrupts() {
        let mut reader = InterruptedThenFailingReader { calls: 0 };
        let err = RingSha256Hasher::new()
            .hash_stream(&mut reader)
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::StreamRead {
                message: "simulated stream failure".to_string(),
            }
        );
        assert_eq!(reader.calls, 2);
    }

    #[test]
    fn digest_matches_sha256sum_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("interop.bin");
        let data = pseudo_random_bytes(123_457);
        std::fs::write(&path, &data).unwrap();

        let output = std::process::Command::new("sha256sum")
            .arg(&path)
            .output()
            .expect("sha256sum must be runnable");
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let expected = stdout.split_whitespace().next().unwrap();

        let digest = RingSha256Hasher::new().hash_bytes(&data);
        assert_eq!(digest.to_hex(), expected);
    }

    mod properties {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn equal_input_gives_equal_digest(
                data in proptest::collection::vec(any::<u8>(), 0..4096),
            ) {
                let hasher = RingSha256Hasher::new();
                prop_assert_eq!(hasher.hash_bytes(&data), hasher.hash_bytes(&data));
                prop_assert_eq!(
                    hasher.hash_bytes(&data),
                    hasher.hash_stream(&mut &data[..]).unwrap()
                );
            }

            #[test]
            fn flipping_one_bit_changes_the_digest(
                data in proptest::collection::vec(any::<u8>(), 1..4096),
                index in any::<proptest::sample::Index>(),
                bit in 0u32..8,
            ) {
                let mut flipped = data.clone();
                let at = index.index(data.len());
                flipped[at] ^= 1 << bit;
                let hasher = RingSha256Hasher::new();
                prop_assert_ne!(hasher.hash_bytes(&data), hasher.hash_bytes(&flipped));
            }
        }
    }
}
