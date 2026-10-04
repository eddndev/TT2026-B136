//! Typed first-factor proof under an exact supplied internal trust snapshot.
//!
//! Publication, current binding ownership, freshness, one-use challenge storage
//! and subsequent MFA are application duties. See
//! docs/adr/0068-owner-certificate-first-factor.md.

use application::credential_trust::CredentialTrustSnapshot;
use domain::{
    crypto::{CredentialCheck, CredentialFailure, DocumentHasher, Signature},
    owner_certificate_login::LoginStatement,
};

use super::internal_profile::{Profile, Verifier};
use crate::RingSha256Hasher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OwnerLoginFailure {
    #[error("owner login credential verification failed")]
    Credential(#[from] CredentialFailure),
    #[error("owner login material does not match supplied trust")]
    MaterialMismatch,
    #[error("owner login challenge window is not live")]
    InvalidWindow,
}

/// Checks a Partner proof without creating a challenge or authenticated session.
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalRsaOwnerLoginVerifier;

impl InternalRsaOwnerLoginVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Checks the challenge window and hashes its exact typed bytes once.
    ///
    /// Credential validity in the result remains independent of challenge expiry:
    /// the latter bounds proof presentation, not every subsequent session lifetime.
    pub fn verify_login(
        &self,
        statement: &LoginStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, OwnerLoginFailure> {
        if !statement.is_live_at(at) {
            return Err(OwnerLoginFailure::InvalidWindow);
        }
        let digest = RingSha256Hasher.hash_bytes(&statement.canonical_bytes());
        let check = Verifier::new(Profile::Partner).verify(
            &digest,
            certificate,
            signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )?;
        let material = statement.material();
        if check.trust != trust.inspection
            || material.deployment() != trust.deployment_id
            || material.trust_revision() != trust.revision.get()
            || material.root() != check.trust.root_fingerprint
            || material.leaf() != check.certificate.fingerprint
        {
            return Err(OwnerLoginFailure::MaterialMismatch);
        }
        Ok(check)
    }
}

impl application::identity::certificate_login::OwnerLoginVerifier
    for InternalRsaOwnerLoginVerifier
{
    fn verify_login(
        &self,
        statement: &LoginStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, application::ApplicationError> {
        InternalRsaOwnerLoginVerifier::verify_login(
            self,
            statement,
            certificate,
            signature,
            trust,
            at,
        )
        .map_err(|_| application::ApplicationError::InvalidCredentials)
    }
}
