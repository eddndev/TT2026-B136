//! Outbound ports for X.509 certificate validation and for the internal
//! certificate authority that issues, revokes, and lists certificates.
//!
//! The trust model is a single-layer chain: one self-signed root authority
//! signs end-entity certificates directly. Certificates, issuers, and
//! revocation lists cross these ports as opaque byte buffers (PEM or DER);
//! parsing lives in the adapters so this crate stays free of X.509
//! libraries. Evaluation time is always an explicit unix timestamp, so
//! callers and tests control the clock instead of reading it implicitly.

use std::fmt;
use std::path::PathBuf;

use crate::error::DomainError;

/// Outcome of validating an end-entity certificate against its issuer.
///
/// Exactly one outcome is reported even when several causes apply at once;
/// adapters check in a fixed order: issuer trust first, then the validity
/// window, then revocation. A revocation list that cannot be trusted or a
/// byte buffer that cannot be parsed is an error, not an outcome; see
/// [`CertificateValidator::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateValidation {
    /// The certificate chains to the issuer, the evaluation time is inside
    /// the validity window, and the serial is absent from the revocation
    /// list when one was supplied.
    Valid,
    /// The evaluation time is after the end of the validity window.
    Expired,
    /// The evaluation time is before the start of the validity window.
    NotYetValid,
    /// The supplied revocation list contains the certificate's serial.
    Revoked {
        /// Serial number matched on the revocation list, in uppercase hex
        /// without leading zeros.
        serial_hex: String,
    },
    /// The certificate does not chain to the presented issuer: the issuer
    /// name does not match the issuer's subject, or the signature over the
    /// certificate body failed to verify with the issuer's public key, or
    /// the evaluation time falls outside the issuer's own validity window,
    /// so the issuer cannot anchor trust at that time.
    UntrustedIssuer,
}

impl fmt::Display for CertificateValidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => f.write_str("valid"),
            Self::Expired => f.write_str("expired"),
            Self::NotYetValid => f.write_str("not yet valid"),
            Self::Revoked { serial_hex } => write!(f, "revoked (serial {serial_hex})"),
            Self::UntrustedIssuer => f.write_str("untrusted issuer"),
        }
    }
}

/// Identity summary of a parsed certificate, for display purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateSummary {
    /// Subject distinguished name, one line.
    pub subject: String,
    /// Issuer distinguished name, one line.
    pub issuer: String,
    /// Serial number in uppercase hex without leading zeros.
    pub serial_hex: String,
    /// Start of the validity window, unix seconds.
    pub not_before_unix: i64,
    /// End of the validity window, unix seconds.
    pub not_after_unix: i64,
}

/// Outbound port: chain validation and inspection of X.509 certificates.
pub trait CertificateValidator {
    /// Validates `certificate` against `issuer` at the given unix time.
    ///
    /// Both byte buffers may be PEM or DER. Revocation contract: when `crl`
    /// is `Some`, revocation checking is mandatory and any problem with the
    /// list (unparseable, not signed by the issuer, or already past its
    /// next-update time) is a hard error rather than a lenient pass. When
    /// `crl` is `None`, no revocation check happens at all; passing `None`
    /// is the caller's explicit decision that validating without a
    /// revocation list is acceptable for its purpose.
    ///
    /// # Errors
    ///
    /// - [`DomainError::MalformedCertificate`] when the certificate or the
    ///   issuer bytes cannot be parsed.
    /// - [`DomainError::MalformedCrl`] when the revocation list bytes
    ///   cannot be parsed.
    /// - [`DomainError::UntrustedCrl`] when the revocation list is not
    ///   signed by the presented issuer.
    /// - [`DomainError::StaleCrl`] when the evaluation time is past the
    ///   revocation list's next scheduled update.
    fn validate(
        &self,
        certificate: &[u8],
        issuer: &[u8],
        crl: Option<&[u8]>,
        unix_seconds: i64,
    ) -> Result<CertificateValidation, DomainError>;

    /// Parses `certificate` (PEM or DER) into its display summary.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::MalformedCertificate`] when the bytes cannot
    /// be parsed.
    fn inspect(&self, certificate: &[u8]) -> Result<CertificateSummary, DomainError>;
}

/// Material produced when the certificate authority issues a certificate.
///
/// The private key is written by the authority to `private_key_path` with
/// restrictive permissions and is never loaded into memory here; only its
/// location is reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedCertificate {
    /// The issued certificate, PEM encoded.
    pub certificate_pem: Vec<u8>,
    /// Where the authority stored the certificate.
    pub certificate_path: PathBuf,
    /// Where the authority stored the subject's private key.
    pub private_key_path: PathBuf,
    /// Serial number in uppercase hex without leading zeros.
    pub serial_hex: String,
}

/// Outbound port: operations of the internal certificate authority.
///
/// Every operation may fail with
/// [`DomainError::CertificateAuthorityFailure`] when the backing authority
/// reports an error.
pub trait CertificateAuthority {
    /// Creates the root authority if it does not exist yet and returns the
    /// root certificate, PEM encoded. Calling it again on an initialized
    /// authority is a no-op that still returns the root certificate.
    fn init_ca(&self) -> Result<Vec<u8>, DomainError>;

    /// Returns the root certificate, PEM encoded.
    fn root_certificate(&self) -> Result<Vec<u8>, DomainError>;

    /// Issues an end-entity certificate for `common_name`.
    fn issue(&self, common_name: &str) -> Result<IssuedCertificate, DomainError>;

    /// Revokes the issued certificate with the given serial (uppercase hex,
    /// as reported by [`CertificateAuthority::issue`]). The revocation only
    /// reaches relying parties after the next
    /// [`CertificateAuthority::generate_crl`].
    fn revoke(&self, serial_hex: &str) -> Result<(), DomainError>;

    /// Regenerates the certificate revocation list and returns it, PEM
    /// encoded.
    fn generate_crl(&self) -> Result<Vec<u8>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test double proving both ports are object safe and exercising the
    /// value types; it performs no real cryptography.
    struct FixedOutcome(CertificateValidation);

    impl CertificateValidator for FixedOutcome {
        fn validate(
            &self,
            _certificate: &[u8],
            _issuer: &[u8],
            crl: Option<&[u8]>,
            unix_seconds: i64,
        ) -> Result<CertificateValidation, DomainError> {
            if crl.is_some() && unix_seconds > 100 {
                return Err(DomainError::StaleCrl {
                    next_update_unix: 100,
                });
            }
            Ok(self.0.clone())
        }

        fn inspect(&self, _certificate: &[u8]) -> Result<CertificateSummary, DomainError> {
            Ok(CertificateSummary {
                subject: "CN=Subject".to_string(),
                issuer: "CN=Issuer".to_string(),
                serial_hex: "1000".to_string(),
                not_before_unix: 0,
                not_after_unix: 100,
            })
        }
    }

    #[test]
    fn validator_port_is_object_safe() {
        let validator: &dyn CertificateValidator = &FixedOutcome(CertificateValidation::Valid);
        let outcome = validator.validate(b"cert", b"issuer", None, 50).unwrap();
        assert_eq!(outcome, CertificateValidation::Valid);
    }

    #[test]
    fn stale_crl_error_carries_the_missed_update_time() {
        let validator = FixedOutcome(CertificateValidation::Valid);
        let err = validator
            .validate(b"cert", b"issuer", Some(b"crl"), 101)
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::StaleCrl {
                next_update_unix: 100,
            }
        );
        assert_eq!(
            err.to_string(),
            "certificate revocation list is stale: next update was due at unix time 100"
        );
    }

    #[test]
    fn outcomes_display_their_cause() {
        assert_eq!(CertificateValidation::Valid.to_string(), "valid");
        assert_eq!(CertificateValidation::Expired.to_string(), "expired");
        assert_eq!(
            CertificateValidation::NotYetValid.to_string(),
            "not yet valid"
        );
        assert_eq!(
            CertificateValidation::Revoked {
                serial_hex: "10A3".to_string(),
            }
            .to_string(),
            "revoked (serial 10A3)"
        );
        assert_eq!(
            CertificateValidation::UntrustedIssuer.to_string(),
            "untrusted issuer"
        );
    }

    #[test]
    fn certificate_errors_display_their_detail() {
        assert_eq!(
            DomainError::MalformedCertificate("not der".to_string()).to_string(),
            "certificate cannot be parsed: not der"
        );
        assert_eq!(
            DomainError::MalformedCrl("truncated".to_string()).to_string(),
            "certificate revocation list cannot be parsed: truncated"
        );
        assert_eq!(
            DomainError::UntrustedCrl.to_string(),
            "certificate revocation list is not signed by the issuer"
        );
        assert_eq!(
            DomainError::CertificateAuthorityFailure("script exited".to_string()).to_string(),
            "certificate authority operation failed: script exited"
        );
    }

    #[test]
    fn summary_reports_the_validity_window() {
        let summary = FixedOutcome(CertificateValidation::Valid)
            .inspect(b"cert")
            .unwrap();
        assert_eq!(summary.subject, "CN=Subject");
        assert_eq!(summary.issuer, "CN=Issuer");
        assert_eq!(summary.serial_hex, "1000");
        assert!(summary.not_before_unix < summary.not_after_unix);
    }
}
