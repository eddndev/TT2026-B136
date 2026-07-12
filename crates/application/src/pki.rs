//! Use cases for the internal public key infrastructure: initialize the
//! authority, issue and revoke certificates, publish the revocation list,
//! and validate or inspect certificates.
//!
//! Every use case takes time as an explicit unix timestamp where time
//! matters, so callers own the clock and tests never sleep.

use domain::crypto::certificate::{
    CertificateAuthority, CertificateSummary, CertificateValidation, CertificateValidator,
    IssuedCertificate,
};

use crate::error::ApplicationError;

/// Creates the root certificate authority (idempotent) and returns the
/// root certificate, PEM encoded.
pub struct InitializeCa<A> {
    authority: A,
}

impl<A: CertificateAuthority> InitializeCa<A> {
    pub fn new(authority: A) -> Self {
        Self { authority }
    }

    /// Returns the root certificate, PEM encoded.
    pub fn execute(&self) -> Result<Vec<u8>, ApplicationError> {
        Ok(self.authority.init_ca()?)
    }
}

/// Issues an end-entity certificate and refuses to report success until
/// the fresh certificate parses and chains to the authority's root.
///
/// The post-issuance check runs without a revocation list: a certificate
/// issued this instant cannot be on a list published earlier.
pub struct IssueCertificate<A, V> {
    authority: A,
    validator: V,
}

impl<A: CertificateAuthority, V: CertificateValidator> IssueCertificate<A, V> {
    pub fn new(authority: A, validator: V) -> Self {
        Self {
            authority,
            validator,
        }
    }

    /// Issues a certificate for `common_name` and verifies it at the
    /// given evaluation time, normally the caller's current time.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::IssuedCertificateInvalid`] when the
    /// issued certificate does not validate against the root.
    pub fn execute(
        &self,
        common_name: &str,
        unix_seconds: i64,
    ) -> Result<IssuedCertificate, ApplicationError> {
        let issued = self.authority.issue(common_name)?;
        let root = self.authority.root_certificate()?;
        let outcome =
            self.validator
                .validate(&issued.certificate_pem, &root, None, unix_seconds)?;
        if outcome != CertificateValidation::Valid {
            return Err(ApplicationError::IssuedCertificateInvalid(
                outcome.to_string(),
            ));
        }
        Ok(issued)
    }
}

/// Revokes an issued certificate by its serial number.
///
/// The revocation reaches relying parties only after [`GenerateCrl`]
/// publishes a fresh revocation list.
pub struct RevokeCertificate<A> {
    authority: A,
}

impl<A: CertificateAuthority> RevokeCertificate<A> {
    pub fn new(authority: A) -> Self {
        Self { authority }
    }

    /// Revokes the certificate whose serial (uppercase hex) is given.
    pub fn execute(&self, serial_hex: &str) -> Result<(), ApplicationError> {
        Ok(self.authority.revoke(serial_hex)?)
    }
}

/// Regenerates the certificate revocation list.
pub struct GenerateCrl<A> {
    authority: A,
}

impl<A: CertificateAuthority> GenerateCrl<A> {
    pub fn new(authority: A) -> Self {
        Self { authority }
    }

    /// Returns the fresh revocation list, PEM encoded.
    pub fn execute(&self) -> Result<Vec<u8>, ApplicationError> {
        Ok(self.authority.generate_crl()?)
    }
}

/// Validates a certificate against an issuer, optionally checking a
/// revocation list.
///
/// The revocation contract is the validator port's: a supplied list is
/// checked strictly, while `None` skips revocation checking entirely and
/// is the caller's explicit choice.
pub struct ValidateCertificate<V> {
    validator: V,
}

impl<V: CertificateValidator> ValidateCertificate<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }

    pub fn execute(
        &self,
        certificate: &[u8],
        issuer: &[u8],
        crl: Option<&[u8]>,
        unix_seconds: i64,
    ) -> Result<CertificateValidation, ApplicationError> {
        Ok(self
            .validator
            .validate(certificate, issuer, crl, unix_seconds)?)
    }
}

/// Parses a certificate into its display summary: subject, issuer,
/// serial, and validity window.
pub struct InspectCertificate<V> {
    validator: V,
}

impl<V: CertificateValidator> InspectCertificate<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }

    pub fn execute(&self, certificate: &[u8]) -> Result<CertificateSummary, ApplicationError> {
        Ok(self.validator.inspect(certificate)?)
    }
}
