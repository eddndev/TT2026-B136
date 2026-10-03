//! Restricted public-certificate verification for internal declarations.

use domain::crypto::{
    CredentialCertificate, CredentialCheck, CredentialFailure, CredentialTrustInspection,
    InternalDeclarationVerifier, Sha256Digest, Signature,
};

use super::internal_profile::{Profile, Verifier};

/// Accepts only the explicit internal declaration certificate and CRL profile.
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalRsaDeclarationVerifier;

impl InternalRsaDeclarationVerifier {
    pub fn new() -> Self {
        Self
    }
}

impl InternalDeclarationVerifier for InternalRsaDeclarationVerifier {
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        Verifier::new(Profile::Declaration).inspect_certificate(certificate)
    }

    fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure> {
        Verifier::new(Profile::Declaration).inspect_trust(root, crl, at)
    }

    fn verify(
        &self,
        statement_digest: &Sha256Digest,
        certificate: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure> {
        Verifier::new(Profile::Declaration).verify(
            statement_digest,
            certificate,
            signature,
            root,
            crl,
            at,
        )
    }
}
