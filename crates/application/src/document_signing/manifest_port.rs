//! Time-stamping of the evidence manifest inside the acceptance operation.
//!
//! Rationale: the section "Generation inside the acceptance operation" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use domain::crypto::Sha256Digest;

use crate::ApplicationError;

/// A verified time-stamp token and the instants acceptance relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenFacts {
    /// The exact token bytes as the authority returned them.
    pub token: Vec<u8>,
    /// `genTime` of the token, Unix seconds.
    pub gen_time: i64,
    /// Whole seconds of the token accuracy, rounded up; 0 when absent.
    pub accuracy_seconds: i64,
}

/// The credential trust material that must have issued the time-stamping
/// certificate and whose CRL is captured as `tsa-crl.pem`.
#[derive(Debug, Clone, Copy)]
pub struct TsaAuthority<'a> {
    pub root_der: &'a [u8],
    pub crl_der: &'a [u8],
}

/// Requests and checks the time-stamp over a manifest digest.
pub trait ManifestTimestamping: Send + Sync {
    /// The facts of a document token that the processor already verified
    /// over `digest`.
    fn facts(&self, token: &[u8], digest: &Sha256Digest) -> Result<TokenFacts, ApplicationError>;

    /// Requests exactly one token over `digest`, never retrying, and verifies
    /// it against `anchor_pem`. The token certificate must be issued by
    /// `authority.root_der`, valid over `genTime` plus or minus the accuracy,
    /// and absent from `authority.crl_der`, a CRL of at most 1 MiB issued by
    /// that root whose window covers the same interval. An authority
    /// failure or a token that does not verify is
    /// [`ApplicationError::TimestampEvidenceRejected`] or a domain timestamp
    /// error; a certificate or CRL that does not meet these conditions is
    /// [`ApplicationError::InvalidConfiguration`].
    fn stamp(
        &self,
        digest: &Sha256Digest,
        anchor_pem: &[u8],
        authority: &TsaAuthority<'_>,
    ) -> Result<TokenFacts, ApplicationError>;
}
