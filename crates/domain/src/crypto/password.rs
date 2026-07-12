//! Outbound port for one-way credential hashing.
//!
//! Passwords and recovery codes are never stored in clear form. An adapter
//! hashes them into PHC strings (the self-describing `$argon2id$...` format)
//! and later checks candidate values against those strings. Verification
//! distinguishes a well-formed hash that simply does not match from a stored
//! hash that cannot be parsed at all, so a caller can treat the latter as
//! data corruption instead of a failed login attempt.

use crate::error::DomainError;

/// Result of checking a candidate credential against a stored PHC string.
///
/// A mismatch is a normal, expected outcome and therefore not an error; it
/// stays distinct from the error path so future lockout policies can count
/// rejected attempts without conflating them with backend failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordVerification {
    /// The candidate matches the stored hash.
    Match,
    /// The stored hash is well formed but the candidate does not match.
    Mismatch,
}

/// Outbound port: one-way hashing and verification of credentials.
pub trait PasswordHasher {
    /// Hashes a credential into a PHC string.
    ///
    /// Every call must draw a fresh random salt, so hashing the same input
    /// twice yields two different strings that both verify.
    fn hash(&self, password: &str) -> Result<String, DomainError>;

    /// Checks a candidate credential against a stored PHC string.
    ///
    /// Returns [`DomainError::MalformedPasswordHash`] when the stored string
    /// cannot be parsed; a parse failure is never reported as a mismatch.
    fn verify(&self, password: &str, stored_phc: &str)
        -> Result<PasswordVerification, DomainError>;
}
