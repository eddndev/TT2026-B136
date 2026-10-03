//! Cryptographic consistency of an Owner registration and supplied trust.
//!
//! The caller must select current published trust and recheck authorization and
//! validity at commit. This verifier neither authenticates an Owner nor proves
//! snapshot publication. See docs/adr/0067-owner-certificate-bindings.md.

use application::credential_trust::CredentialTrustSnapshot;
use domain::{
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher, Signature,
    },
    owner_certificates::BindingStatement,
};

use super::internal_profile::{Profile, Verifier};
use crate::RingSha256Hasher;

/// Neutral rejections contain no submitted identifiers, material or parser text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OwnerBindingFailure {
    #[error("owner certificate credential verification failed")]
    Credential(#[from] CredentialFailure),
    #[error("owner certificate binding material does not match supplied trust")]
    MaterialMismatch,
}

/// Admits only the internal Partner profile for an exact typed registration.
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalRsaOwnerBindingVerifier;

impl InternalRsaOwnerBindingVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Inspects bounded public material without establishing issuer trust or time.
    pub fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        Verifier::new(Profile::Partner).inspect_certificate(certificate)
    }

    /// Hashes the canonical registration once and requires its exact trust material.
    ///
    /// The supplied snapshot is mandatory. Its complete derived inspection is
    /// recomputed; its database provenance remains the authenticated caller's duty.
    pub fn verify_registration(
        &self,
        statement: &BindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, OwnerBindingFailure> {
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
            return Err(OwnerBindingFailure::MaterialMismatch);
        }
        Ok(check)
    }
}

impl application::identity::owner_certificates::OwnerBindingVerifier
    for InternalRsaOwnerBindingVerifier
{
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        InternalRsaOwnerBindingVerifier::inspect_certificate(self, certificate)
    }

    fn verify_registration(
        &self,
        statement: &BindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, application::identity::owner_certificates::OwnerCertificateError>
    {
        use application::identity::owner_certificates::OwnerCertificateError;
        InternalRsaOwnerBindingVerifier::verify_registration(
            self,
            statement,
            certificate,
            signature,
            trust,
            at,
        )
        .map_err(|error| match error {
            OwnerBindingFailure::Credential(value) => OwnerCertificateError::Credential(value),
            OwnerBindingFailure::MaterialMismatch => OwnerCertificateError::MaterialMismatch,
        })
    }
}
