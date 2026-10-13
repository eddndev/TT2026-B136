//! Exact internal leaf profiles for administrative public-request issuance.

use domain::crypto::{
    certificate::{CertificateRequestProfile, CertificateRequestProfileValidator},
    CredentialFailure,
};

use super::internal_profile::{Profile, Verifier};

/// Reuses the same strict profiles admitted by their application workflows.
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalRsaCertificateRequestProfileValidator;

impl InternalRsaCertificateRequestProfileValidator {
    pub fn new() -> Self {
        Self
    }
}

impl CertificateRequestProfileValidator for InternalRsaCertificateRequestProfileValidator {
    fn validate_profile(
        &self,
        certificate: &[u8],
        profile: CertificateRequestProfile,
    ) -> Result<(), CredentialFailure> {
        let profile = match profile {
            CertificateRequestProfile::DocumentSigning => Profile::Declaration,
            CertificateRequestProfile::Partner => Profile::Partner,
        };
        Verifier::new(profile)
            .inspect_certificate(certificate)
            .map(|_| ())
    }
}
