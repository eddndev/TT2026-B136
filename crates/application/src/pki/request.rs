//! Administrative public-request issuance with the fixed signing-only profile.

use domain::crypto::certificate::{
    CertificateAuthority, CertificateRequestAuthority, CertificateValidation, CertificateValidator,
    IssuedPublicCertificate,
};
use domain::crypto::InternalDeclarationVerifier;

use crate::ApplicationError;

/// Issues public evidence without a holder private-key input or output.
///
/// Request proof and exact public-key correspondence belong to the authority
/// port; chain, validity and the admitted leaf profile are checked before success.
pub struct IssueCertificateRequest<A, V, D> {
    authority: A,
    validator: V,
    declarations: D,
}

impl<A, V, D> IssueCertificateRequest<A, V, D>
where
    A: CertificateAuthority + CertificateRequestAuthority,
    V: CertificateValidator,
    D: InternalDeclarationVerifier,
{
    pub fn new(authority: A, validator: V, declarations: D) -> Self {
        Self {
            authority,
            validator,
            declarations,
        }
    }

    pub fn execute(
        &self,
        name: &str,
        request_pem: &[u8],
        unix_seconds: i64,
    ) -> Result<IssuedPublicCertificate, ApplicationError> {
        let issued = self.authority.issue_request(name, request_pem)?;
        let root = self.authority.root_certificate()?;
        let outcome =
            self.validator
                .validate(&issued.certificate_pem, &root, None, unix_seconds)?;
        if outcome != CertificateValidation::Valid {
            return Err(ApplicationError::IssuedCertificateInvalid(
                outcome.to_string(),
            ));
        }
        self.declarations
            .inspect_certificate(&issued.certificate_pem)
            .map_err(|error| ApplicationError::IssuedCertificateInvalid(error.to_string()))?;
        Ok(issued)
    }
}
