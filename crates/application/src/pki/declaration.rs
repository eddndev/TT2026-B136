//! Issuance with the internal declaration leaf profile enforced before success.

use domain::crypto::certificate::{CertificateAuthority, CertificateValidator, IssuedCertificate};
use domain::crypto::InternalDeclarationVerifier;

use super::IssueCertificate;
use crate::ApplicationError;

pub struct IssueDeclarationCertificate<A, V, D> {
    issuer: IssueCertificate<A, V>,
    declarations: D,
}

impl<A, V, D> IssueDeclarationCertificate<A, V, D>
where
    A: CertificateAuthority,
    V: CertificateValidator,
    D: InternalDeclarationVerifier,
{
    pub fn new(authority: A, validator: V, declarations: D) -> Self {
        Self {
            issuer: IssueCertificate::new(authority, validator),
            declarations,
        }
    }

    pub fn execute(
        &self,
        common_name: &str,
        unix_seconds: i64,
    ) -> Result<IssuedCertificate, ApplicationError> {
        let issued = self.issuer.execute(common_name, unix_seconds)?;
        self.declarations
            .inspect_certificate(&issued.certificate_pem)
            .map_err(|error| ApplicationError::IssuedCertificateInvalid(error.to_string()))?;
        Ok(issued)
    }
}
