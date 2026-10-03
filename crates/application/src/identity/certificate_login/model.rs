use domain::{
    crypto::{CredentialCertificate, Sha256Digest},
    owner_certificate_login::LoginStatement,
    owner_certificates::Uuid,
};

use crate::{
    credential_trust::CredentialTrustSnapshot,
    identity::{owner_certificates::OwnerBindingAccount, LoginChallengeIdentity, Principal},
};

/// Current live binding and published trust loaded by a trusted server adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateLoginContext {
    pub account: OwnerBindingAccount,
    pub binding_id: Uuid,
    pub certificate: CredentialCertificate,
    pub trust: CredentialTrustSnapshot,
}

/// Immutable public capture consumed before the first cryptographic attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCertificateLogin {
    pub context: CertificateLoginContext,
    pub statement: LoginStatement,
}

/// An unsigned first-factor challenge; it authenticates no principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateLoginChallenge {
    pub challenge_token: String,
    pub statement: LoginStatement,
    pub expires_in_seconds: u64,
}

/// Verified origin retained independently of the nonce presentation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateSessionProvenance {
    pub principal: Principal,
    pub auth_generation: u64,
    pub binding_id: Uuid,
    pub deployment_id: Uuid,
    pub trust_revision: u32,
    pub root_fingerprint: Sha256Digest,
    pub leaf_fingerprint: Sha256Digest,
    pub crl_digest: Sha256Digest,
    pub valid_from_unix_seconds: i64,
    pub valid_until_unix_seconds: i64,
}

/// Certificate proof awaiting mandatory MFA within its own bounded window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateMfaChallenge {
    pub identity: LoginChallengeIdentity,
    pub provenance: CertificateSessionProvenance,
    pub expires_at_unix_seconds: i64,
}

/// Strictly distinguished records returned by an atomic challenge claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MfaChallenge {
    Password(LoginChallengeIdentity),
    Certificate(Box<CertificateMfaChallenge>),
}

/// Explicit session origin; absence is never interpreted as Password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAuthentication {
    Password,
    Certificate(Box<CertificateSessionProvenance>),
}
