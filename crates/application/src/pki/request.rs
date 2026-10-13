//! Administrative public-request issuance with an explicit internal leaf profile.

use domain::crypto::certificate::{
    CertificateAuthority, CertificateRequestAuthority, CertificateRequestProfile,
    CertificateRequestProfileValidator, CertificateValidation, CertificateValidator,
    IssuedPublicCertificate,
};

use crate::ApplicationError;

/// Issues public evidence without a holder private-key input or output.
///
/// Request proof and exact public-key correspondence belong to the authority
/// port; chain, validity and the admitted leaf profile are checked before success.
pub struct IssueCertificateRequest<A, V, P> {
    authority: A,
    validator: V,
    profiles: P,
}

impl<A, V, P> IssueCertificateRequest<A, V, P>
where
    A: CertificateAuthority + CertificateRequestAuthority,
    V: CertificateValidator,
    P: CertificateRequestProfileValidator,
{
    pub fn new(authority: A, validator: V, profiles: P) -> Self {
        Self {
            authority,
            validator,
            profiles,
        }
    }

    pub fn execute(
        &self,
        name: &str,
        request_pem: &[u8],
        profile: CertificateRequestProfile,
        unix_seconds: i64,
    ) -> Result<IssuedPublicCertificate, ApplicationError> {
        let issued = self.authority.issue_request(name, request_pem, profile)?;
        let root = self.authority.root_certificate()?;
        let outcome =
            self.validator
                .validate(&issued.certificate_pem, &root, None, unix_seconds)?;
        if outcome != CertificateValidation::Valid {
            return Err(ApplicationError::IssuedCertificateInvalid(
                outcome.to_string(),
            ));
        }
        self.profiles
            .validate_profile(&issued.certificate_pem, profile)
            .map_err(|error| ApplicationError::IssuedCertificateInvalid(error.to_string()))?;
        Ok(issued)
    }
}
