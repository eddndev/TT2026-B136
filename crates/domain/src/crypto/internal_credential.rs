//! Public evidence and verification port for internal declaration credentials.

use std::fmt;

use super::{CertificateSummary, Sha256Digest, Signature};

/// Canonical public certificate material; subject names are omitted from Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialCertificate {
    pub der: Vec<u8>,
    pub fingerprint: Sha256Digest,
    pub summary: CertificateSummary,
}

/// A complete, signed root and CRL pair admitted by the internal profile.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialTrustInspection {
    pub root_der: Vec<u8>,
    pub crl_der: Vec<u8>,
    pub root_fingerprint: Sha256Digest,
    pub crl_digest: Sha256Digest,
    pub crl_number: u64,
    pub crl_this_update: i64,
    pub crl_next_update: i64,
    pub valid_from: i64,
    pub valid_until: i64,
}

/// Captured verification, distinct from a later transaction acceptance time.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialCheck {
    pub certificate: CredentialCertificate,
    pub trust: CredentialTrustInspection,
    pub statement_digest: Sha256Digest,
    pub signature: Signature,
    pub checked_at: i64,
    pub valid_from: i64,
    pub valid_until: i64,
}

/// Rejections contain no submitted identifiers, bytes, or parser diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialFailure {
    LimitExceeded,
    MalformedCertificate,
    UnsupportedCertificate,
    UntrustedIssuer,
    NotYetValid,
    Expired,
    MalformedCrl,
    UnsupportedCrl,
    UntrustedCrl,
    CrlNotYetValid,
    CrlExpired,
    Revoked,
    InvalidSignature,
}

impl fmt::Display for CredentialFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LimitExceeded => "credential material exceeds its limit",
            Self::MalformedCertificate => "certificate is malformed",
            Self::UnsupportedCertificate => "certificate does not match the internal profile",
            Self::UntrustedIssuer => "certificate issuer is not trusted",
            Self::NotYetValid => "certificate is not yet valid",
            Self::Expired => "certificate has expired",
            Self::MalformedCrl => "revocation list is malformed",
            Self::UnsupportedCrl => "revocation list does not match the internal profile",
            Self::UntrustedCrl => "revocation list issuer is not trusted",
            Self::CrlNotYetValid => "revocation list is not yet valid",
            Self::CrlExpired => "revocation list has expired",
            Self::Revoked => "certificate is revoked",
            Self::InvalidSignature => "declaration signature is invalid",
        })
    }
}

impl std::error::Error for CredentialFailure {}

/// Verifies public materials only, with mandatory revocation and explicit time.
///
/// The caller constructs and hashes the canonical declaration. A valid result
/// establishes a signature under the internal authority, not a legal identity.
pub trait InternalDeclarationVerifier: Send + Sync {
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure>;

    fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure>;

    fn verify(
        &self,
        digest: &Sha256Digest,
        certificate: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure>;
}

impl fmt::Debug for CredentialCertificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialCertificate")
            .field("der_length", &self.der.len())
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CredentialTrustInspection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialTrustInspection")
            .field("root_fingerprint", &self.root_fingerprint)
            .field("crl_digest", &self.crl_digest)
            .field("crl_number", &self.crl_number)
            .field("crl_this_update", &self.crl_this_update)
            .field("crl_next_update", &self.crl_next_update)
            .field("valid_from", &self.valid_from)
            .field("valid_until", &self.valid_until)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CredentialCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialCheck")
            .field("certificate", &self.certificate)
            .field("trust", &self.trust)
            .field("statement_digest", &self.statement_digest)
            .field("checked_at", &self.checked_at)
            .field("valid_from", &self.valid_from)
            .field("valid_until", &self.valid_until)
            .finish_non_exhaustive()
    }
}
