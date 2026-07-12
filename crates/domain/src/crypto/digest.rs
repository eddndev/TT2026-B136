//! A fixed-length SHA-256 content digest.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Number of bytes in a SHA-256 digest.
pub const SHA256_LEN: usize = 32;

/// A SHA-256 digest of some content.
///
/// The value is always exactly [`SHA256_LEN`] bytes. Build it from raw bytes
/// produced by a hashing adapter, or decode it from its lower-case hex form.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256Digest([u8; SHA256_LEN]);

impl Sha256Digest {
    /// Wraps an exact 32-byte array.
    pub const fn from_array(bytes: [u8; SHA256_LEN]) -> Self {
        Self(bytes)
    }

    /// Builds a digest from a byte slice, rejecting the wrong length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        let array: [u8; SHA256_LEN] =
            bytes
                .try_into()
                .map_err(|_| DomainError::InvalidDigestLength {
                    expected: SHA256_LEN,
                    actual: bytes.len(),
                })?;
        Ok(Self(array))
    }

    /// Borrows the raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; SHA256_LEN] {
        &self.0
    }

    /// Renders the digest as a lower-case hex string.
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(SHA256_LEN * 2);
        for byte in self.0 {
            out.push(hex_nibble(byte >> 4));
            out.push(hex_nibble(byte & 0x0f));
        }
        out
    }

    /// Parses a lower- or upper-case hex string of exactly 64 characters.
    pub fn from_hex(text: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::InvalidHexEncoding {
            expected: SHA256_LEN,
        };
        if text.len() != SHA256_LEN * 2 {
            return Err(invalid());
        }
        let mut bytes = [0u8; SHA256_LEN];
        let raw = text.as_bytes();
        for (i, slot) in bytes.iter_mut().enumerate() {
            let high = decode_nibble(raw[i * 2]).ok_or_else(invalid)?;
            let low = decode_nibble(raw[i * 2 + 1]).ok_or_else(invalid)?;
            *slot = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + (value - 10)) as char,
    }
}

fn decode_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Digest({})", self.to_hex())
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> [u8; SHA256_LEN] {
        let mut bytes = [0u8; SHA256_LEN];
        for (i, slot) in bytes.iter_mut().enumerate() {
            *slot = i as u8;
        }
        bytes
    }

    #[test]
    fn from_bytes_accepts_exact_length() {
        let digest = Sha256Digest::from_bytes(&sample()).unwrap();
        assert_eq!(digest.as_bytes(), &sample());
    }

    #[test]
    fn from_bytes_rejects_wrong_length() {
        let err = Sha256Digest::from_bytes(&[0u8; 31]).unwrap_err();
        assert_eq!(
            err,
            DomainError::InvalidDigestLength {
                expected: 32,
                actual: 31,
            }
        );
    }

    #[test]
    fn hex_round_trip_is_stable() {
        let digest = Sha256Digest::from_array(sample());
        let restored = Sha256Digest::from_hex(&digest.to_hex()).unwrap();
        assert_eq!(digest, restored);
    }

    #[test]
    fn from_hex_rejects_bad_length() {
        assert_eq!(
            Sha256Digest::from_hex("00"),
            Err(DomainError::InvalidHexEncoding { expected: 32 })
        );
    }

    #[test]
    fn from_hex_rejects_non_hex_characters() {
        let text = "z".repeat(64);
        assert_eq!(
            Sha256Digest::from_hex(&text),
            Err(DomainError::InvalidHexEncoding { expected: 32 })
        );
    }

    #[test]
    fn display_matches_known_hex() {
        let digest = Sha256Digest::from_array([0xab; SHA256_LEN]);
        assert_eq!(digest.to_hex(), "ab".repeat(32));
    }
}
