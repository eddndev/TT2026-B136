//! Outbound port for time-based one-time passwords (RFC 6238).
//!
//! The adapter owns algorithm details (HMAC function, digit count, step
//! length); this port fixes the contract: a shared secret is created at
//! enrollment together with a provisioning URI, and a submitted code is
//! checked against a given unix timestamp accepting one time step of clock
//! skew on each side.
//!
//! Secret-carrying fields use `Zeroizing` so they are wiped on drop; see
//! docs/adr/0003-zeroize-secret-material-in-domain.md.

use zeroize::Zeroizing;

use crate::error::DomainError;

/// Result of checking a submitted one-time code.
///
/// A rejected code is a normal outcome, kept apart from the error path so a
/// future lockout policy can count rejections without conflating them with
/// backend failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpVerification {
    /// The code is valid for the timestamp (within one step of skew).
    Accepted,
    /// The code is not valid for the timestamp.
    Rejected,
}

/// Material produced when a user enrolls a second factor.
///
/// Every field derives from the shared secret, so all of them are wiped on
/// drop. They are shown to the user exactly once, at enrollment.
pub struct TotpEnrollment {
    /// Raw shared secret bytes.
    pub secret: Zeroizing<Vec<u8>>,
    /// The secret in base32 (RFC 4648, no padding), as authenticator apps
    /// expect it for manual entry.
    pub secret_base32: Zeroizing<String>,
    /// Provisioning URI (`otpauth://totp/...`) suitable for QR encoding.
    /// It embeds the secret, so it is as sensitive as the secret itself.
    pub otpauth_uri: Zeroizing<String>,
}

/// Outbound port: enrollment and verification of time-based one-time codes.
pub trait TotpProvider {
    /// Draws a fresh random secret and derives the provisioning material
    /// for the given account.
    fn enroll(&self, account_email: &str) -> Result<TotpEnrollment, DomainError>;

    /// Checks a submitted code at the given unix timestamp, accepting the
    /// previous, current, and next time step.
    fn verify(
        &self,
        secret: &[u8],
        code: &str,
        unix_seconds: u64,
    ) -> Result<TotpVerification, DomainError>;

    /// Computes the code for the time step containing the given timestamp.
    /// Exists so tests and diagnostics can produce a known-good code; it
    /// must never be exposed as a login aid.
    fn current_code(&self, secret: &[u8], unix_seconds: u64) -> Result<String, DomainError>;
}
