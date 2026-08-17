//! Argon2id adapter for the credential hashing port.

use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Algorithm, Argon2, Params, PasswordHasher as _, PasswordVerifier, Version};
use domain::crypto::password::{PasswordHasher, PasswordVerification};
use domain::DomainError;

/// Memory cost in KiB (256 MiB), calibrated with four passes on the reference
/// host to target 500-1000 ms per password hash.
const MEMORY_KIB: u32 = 262144;

/// Number of passes over the memory.
const ITERATIONS: u32 = 4;

/// Degree of parallelism (lanes).
const PARALLELISM: u32 = 1;

/// Length in bytes of the derived hash.
const OUTPUT_LEN: usize = 32;

/// Length in bytes of the random salt drawn for every hash.
const SALT_LEN: usize = 16;

/// Hashes credentials with Argon2id and verifies them against PHC strings.
pub struct Argon2idHasher {
    argon2: Argon2<'static>,
}

impl Argon2idHasher {
    /// Builds the hasher with the fixed parameters above.
    pub fn new() -> Self {
        let params = Params::new(MEMORY_KIB, ITERATIONS, PARALLELISM, Some(OUTPUT_LEN))
            .expect("the fixed argon2id parameters are within the allowed ranges");
        Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }
}

impl Default for Argon2idHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasher for Argon2idHasher {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        let mut salt_bytes = [0u8; SALT_LEN];
        getrandom::getrandom(&mut salt_bytes)
            .map_err(|err| DomainError::PasswordHashingFailed(err.to_string()))?;
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|err| DomainError::PasswordHashingFailed(err.to_string()))?;
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|err| DomainError::PasswordHashingFailed(err.to_string()))?;
        Ok(hash.to_string())
    }

    fn verify(
        &self,
        password: &str,
        stored_phc: &str,
    ) -> Result<PasswordVerification, DomainError> {
        let parsed =
            PasswordHash::new(stored_phc).map_err(|_| DomainError::MalformedPasswordHash)?;
        // A PHC string without a salt or without the derived hash parses,
        // but the verifier reports it as a plain password mismatch. Reject
        // it here so an incomplete stored hash is never mistaken for a
        // failed login attempt.
        if parsed.salt.is_none() || parsed.hash.is_none() {
            return Err(DomainError::MalformedPasswordHash);
        }
        match self.argon2.verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(PasswordVerification::Match),
            Err(argon2::password_hash::Error::Password) => Ok(PasswordVerification::Mismatch),
            // Any other failure means the stored string carried parameters or
            // encodings this adapter cannot process; report it as malformed
            // rather than as a mismatch.
            Err(_) => Err(DomainError::MalformedPasswordHash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phc_string_records_the_fixed_argon2id_parameters() {
        let hasher = Argon2idHasher::new();
        let phc = hasher.hash("correct horse battery staple").unwrap();
        assert!(phc.starts_with("$argon2id$v=19$"), "got: {phc}");
        assert!(phc.contains("m=262144,t=4,p=1"), "got: {phc}");
    }

    #[test]
    fn verify_accepts_the_right_password_and_rejects_a_wrong_one() {
        let hasher = Argon2idHasher::new();
        let phc = hasher.hash("right password").unwrap();
        assert_eq!(
            hasher.verify("right password", &phc).unwrap(),
            PasswordVerification::Match
        );
        assert_eq!(
            hasher.verify("wrong password", &phc).unwrap(),
            PasswordVerification::Mismatch
        );
    }

    #[test]
    fn hashing_twice_yields_different_strings_because_salts_are_random() {
        let hasher = Argon2idHasher::new();
        let first = hasher.hash("same password").unwrap();
        let second = hasher.hash("same password").unwrap();
        assert_ne!(first, second);
        assert_eq!(
            hasher.verify("same password", &second).unwrap(),
            PasswordVerification::Match
        );
    }

    #[test]
    fn malformed_stored_hash_is_an_error_not_a_mismatch() {
        let hasher = Argon2idHasher::new();
        for stored in ["", "not a phc string", "$argon2id$v=19$broken"] {
            assert_eq!(
                hasher.verify("anything", stored).unwrap_err(),
                DomainError::MalformedPasswordHash,
                "stored: {stored:?}"
            );
        }
    }
}
