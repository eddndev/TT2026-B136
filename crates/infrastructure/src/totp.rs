//! TOTP adapter backed by the totp-rs crate.
//!
//! Configuration: HMAC-SHA1, six digits, thirty-second steps, one step of
//! accepted clock skew on each side. Code comparison inside the pinned
//! totp-rs release is constant time (its `TOTP::check` compares candidate
//! codes with the `constant_time_eq` crate), so no extra comparison is
//! layered on top.

use domain::crypto::totp::{TotpEnrollment, TotpProvider, TotpVerification};
use domain::DomainError;
use totp_rs::{Algorithm, Secret, TOTP};
use zeroize::Zeroizing;

use crate::error::CryptoError;

/// Issuer recorded in provisioning URIs.
const ISSUER: &str = "despacho";

/// Seconds per time step (the RFC 6238 default).
const STEP_SECONDS: u64 = 30;

/// Accepted clock skew, in steps, on each side of the current step.
const SKEW_STEPS: u8 = 1;

/// Bytes drawn for a fresh shared secret: 160 bits, matching the HMAC-SHA1
/// output size as RFC 4226 recommends.
const SECRET_LEN: usize = 20;

/// Minimum secret length accepted for verification; RFC 4226 requires at
/// least 128 bits.
const MIN_SECRET_LEN: usize = 16;

/// Digits per code in the production configuration.
const DIGITS: usize = 6;

/// Time-based one-time passwords via the totp-rs crate.
pub struct TotpRsProvider {
    digits: usize,
}

impl TotpRsProvider {
    /// Builds the production configuration: six digits.
    pub fn new() -> Self {
        Self { digits: DIGITS }
    }

    /// Same mechanism with a different digit count (6 to 8). The standard
    /// vectors in RFC 6238 appendix B are eight-digit, so tests need this;
    /// production code uses [`TotpRsProvider::new`].
    pub fn with_digits(digits: usize) -> Self {
        debug_assert!((6..=8).contains(&digits), "rfc 4226 allows 6 to 8 digits");
        Self { digits }
    }

    /// Wraps a caller-provided secret in the library type used to compute
    /// and check codes. The library value holds its own copy of the secret
    /// and wipes it on drop (totp-rs `zeroize` feature).
    fn code_engine(&self, secret: &[u8]) -> Result<TOTP, DomainError> {
        if secret.len() < MIN_SECRET_LEN {
            return Err(DomainError::TotpSecretTooShort {
                minimum: MIN_SECRET_LEN,
                actual: secret.len(),
            });
        }
        // `TOTP::new` would re-run this length check and validate label
        // fields that code computation never uses, so the unchecked
        // constructor is safe here.
        Ok(TOTP::new_unchecked(
            Algorithm::SHA1,
            self.digits,
            SKEW_STEPS,
            STEP_SECONDS,
            secret.to_vec(),
            None,
            String::new(),
        ))
    }
}

impl Default for TotpRsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TotpProvider for TotpRsProvider {
    fn enroll(&self, account_email: &str) -> Result<TotpEnrollment, DomainError> {
        let mut secret = Zeroizing::new(vec![0u8; SECRET_LEN]);
        getrandom::getrandom(secret.as_mut_slice())
            .map_err(|err| DomainError::RandomnessFailed(err.to_string()))?;
        let secret_base32 = Zeroizing::new(encode_base32(&secret));
        // A fully labeled value is needed only to derive the provisioning
        // URI; `TOTP::new` also rejects a label containing ':'.
        let labeled = TOTP::new(
            Algorithm::SHA1,
            self.digits,
            SKEW_STEPS,
            STEP_SECONDS,
            secret.to_vec(),
            Some(ISSUER.to_string()),
            account_email.to_string(),
        )
        .map_err(|err| DomainError::TotpBackendFailed(err.to_string()))?;
        let otpauth_uri = Zeroizing::new(labeled.get_url());
        Ok(TotpEnrollment {
            secret,
            secret_base32,
            otpauth_uri,
        })
    }

    fn verify(
        &self,
        secret: &[u8],
        code: &str,
        unix_seconds: u64,
    ) -> Result<TotpVerification, DomainError> {
        // The library's skew arithmetic underflows for timestamps inside
        // the very first time steps after the epoch; real timestamps are
        // decades past that, so reject them instead of panicking.
        if unix_seconds < STEP_SECONDS * u64::from(SKEW_STEPS) {
            return Err(DomainError::TotpBackendFailed(
                "timestamp precedes the first verifiable time step".to_string(),
            ));
        }
        let engine = self.code_engine(secret)?;
        if engine.check(code, unix_seconds) {
            Ok(TotpVerification::Accepted)
        } else {
            Ok(TotpVerification::Rejected)
        }
    }

    fn current_code(&self, secret: &[u8], unix_seconds: u64) -> Result<String, DomainError> {
        Ok(self.code_engine(secret)?.generate(unix_seconds))
    }
}

/// Encodes a secret as base32 (RFC 4648, no padding), the alphabet
/// authenticator apps expect.
fn encode_base32(secret: &[u8]) -> String {
    // The library's `Secret` holds its own copy of the bytes and wipes it
    // on drop (totp-rs `zeroize` feature), which is also why the encoding
    // must be cloned out instead of moved.
    match Secret::Raw(secret.to_vec()).to_encoded() {
        Secret::Encoded(ref text) => text.clone(),
        Secret::Raw(_) => unreachable!("to_encoded always returns the encoded variant"),
    }
}

/// Decodes the base32 form of a secret produced at enrollment.
pub fn decode_base32_secret(text: &str) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    Secret::Encoded(text.trim().to_string())
        .to_bytes()
        .map(Zeroizing::new)
        .map_err(|_| CryptoError::InvalidKeyMaterial("secret is not valid base32".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared secret of the SHA-1 rows in RFC 6238 appendix B and of the
    /// RFC 4226 appendix D vectors: the ASCII bytes "12345678901234567890".
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn eight_digit_codes_match_the_rfc6238_appendix_b_sha1_vectors() {
        let provider = TotpRsProvider::with_digits(8);
        let vectors: [(u64, &str); 6] = [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ];
        for (time, expected) in vectors {
            assert_eq!(
                provider.current_code(RFC_SECRET, time).unwrap(),
                expected,
                "time {time}"
            );
        }
    }

    #[test]
    fn six_digit_codes_match_the_rfc4226_appendix_d_vectors() {
        // TOTP at time T is HOTP at counter T / step, so the six-digit
        // counter-based vectors of RFC 4226 appendix D cross-check the
        // production digit count at time = counter * 30.
        let provider = TotpRsProvider::new();
        let vectors: [&str; 10] = [
            "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583",
            "399871", "520489",
        ];
        for (counter, expected) in vectors.iter().enumerate() {
            let time = counter as u64 * STEP_SECONDS;
            assert_eq!(
                provider.current_code(RFC_SECRET, time).unwrap(),
                *expected,
                "counter {counter}"
            );
        }
    }

    #[test]
    fn verification_accepts_exactly_one_step_of_skew_on_each_side() {
        let provider = TotpRsProvider::new();
        let at = 1_111_111_109u64;
        for offset_steps in [-1i64, 0, 1] {
            let code_time = at.checked_add_signed(offset_steps * STEP_SECONDS as i64);
            let code = provider
                .current_code(RFC_SECRET, code_time.unwrap())
                .unwrap();
            assert_eq!(
                provider.verify(RFC_SECRET, &code, at).unwrap(),
                TotpVerification::Accepted,
                "offset {offset_steps}"
            );
        }
        for offset_steps in [-2i64, 2] {
            let code_time = at.checked_add_signed(offset_steps * STEP_SECONDS as i64);
            let code = provider
                .current_code(RFC_SECRET, code_time.unwrap())
                .unwrap();
            assert_eq!(
                provider.verify(RFC_SECRET, &code, at).unwrap(),
                TotpVerification::Rejected,
                "offset {offset_steps}"
            );
        }
    }

    #[test]
    fn enrollment_produces_a_labeled_uri_and_a_matching_base32_secret() {
        let provider = TotpRsProvider::new();
        let enrollment = provider.enroll("user@example.com").unwrap();
        assert_eq!(enrollment.secret.len(), SECRET_LEN);
        assert!(
            enrollment
                .otpauth_uri
                .starts_with("otpauth://totp/despacho:user%40example.com?"),
            "got: {}",
            enrollment.otpauth_uri.as_str()
        );
        assert!(enrollment
            .otpauth_uri
            .contains(&format!("secret={}", enrollment.secret_base32.as_str())));
        assert!(enrollment.otpauth_uri.contains("issuer=despacho"));
        let decoded = decode_base32_secret(&enrollment.secret_base32).unwrap();
        assert_eq!(decoded.as_slice(), enrollment.secret.as_slice());
    }

    #[test]
    fn two_enrollments_draw_different_secrets() {
        let provider = TotpRsProvider::new();
        let first = provider.enroll("user@example.com").unwrap();
        let second = provider.enroll("user@example.com").unwrap();
        assert_ne!(first.secret.as_slice(), second.secret.as_slice());
    }

    #[test]
    fn a_secret_shorter_than_the_minimum_is_rejected() {
        let provider = TotpRsProvider::new();
        let err = provider.current_code(&[0u8; 8], 59).unwrap_err();
        assert_eq!(
            err,
            DomainError::TotpSecretTooShort {
                minimum: MIN_SECRET_LEN,
                actual: 8,
            }
        );
    }

    #[test]
    fn a_non_base32_secret_encoding_is_rejected() {
        assert!(decode_base32_secret("not base32 at all!").is_err());
    }
}
