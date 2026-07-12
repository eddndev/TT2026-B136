//! X.509 chain validation for the single-layer internal authority.
//!
//! The x509-cert crate family only parses. Signature checking happens
//! here: the parsed to-be-signed structure is re-encoded to DER and the
//! sha256WithRSAEncryption signature is verified with the rsa and sha2
//! crates against the issuer's subject public key. The same technique
//! covers the revocation list's signature over its to-be-signed list.
//! Rationale: docs/adr/0004-certificate-validation-implementation.md.

use der::asn1::ObjectIdentifier;
use der::Encode;
use domain::crypto::certificate::{
    CertificateSummary, CertificateValidation, CertificateValidator,
};
use domain::DomainError;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256};
use x509_cert::certificate::Certificate;

use super::parse::{parse_certificate, parse_crl, serial_hex, time_to_unix};

/// OID of the sha256WithRSAEncryption signature algorithm.
const SHA256_WITH_RSA_ENCRYPTION: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");

/// [`CertificateValidator`] for certificates and revocation lists signed
/// with sha256WithRSAEncryption, the only algorithm the internal authority
/// produces. A certificate signed with any other algorithm cannot chain to
/// the presented issuer and is reported as an untrusted issuer.
///
/// Checks run in a fixed order and the first failing one is reported:
/// issuer trust (name match plus signature), then the certificate's
/// validity window, then the issuer's own validity window, then
/// revocation against the optional list.
#[derive(Debug, Clone, Copy, Default)]
pub struct X509ChainValidator;

impl X509ChainValidator {
    /// Creates the validator; it holds no state.
    pub fn new() -> Self {
        Self
    }
}

impl CertificateValidator for X509ChainValidator {
    fn validate(
        &self,
        certificate: &[u8],
        issuer: &[u8],
        crl: Option<&[u8]>,
        unix_seconds: i64,
    ) -> Result<CertificateValidation, DomainError> {
        let cert = parse_certificate(certificate)?;
        let issuer_cert = parse_certificate(issuer)?;

        if !chains_to_issuer(&cert, &issuer_cert)? {
            return Ok(CertificateValidation::UntrustedIssuer);
        }

        let validity = &cert.tbs_certificate.validity;
        if unix_seconds < time_to_unix(&validity.not_before) {
            return Ok(CertificateValidation::NotYetValid);
        }
        if unix_seconds > time_to_unix(&validity.not_after) {
            return Ok(CertificateValidation::Expired);
        }

        // An issuer outside its own validity window cannot anchor trust at
        // the evaluation time, neither for the certificate nor for any
        // revocation list it signed, so this check must precede the
        // revocation step below. The verdict is UntrustedIssuer rather
        // than Expired or NotYetValid because those two report the
        // validated certificate's own window; `openssl verify -attime`
        // rejects the same chain with a validity error at depth 1.
        let issuer_validity = &issuer_cert.tbs_certificate.validity;
        if unix_seconds < time_to_unix(&issuer_validity.not_before)
            || unix_seconds > time_to_unix(&issuer_validity.not_after)
        {
            return Ok(CertificateValidation::UntrustedIssuer);
        }

        if let Some(crl_bytes) = crl {
            if let Some(serial) = revoked_serial(crl_bytes, &cert, &issuer_cert, unix_seconds)? {
                return Ok(CertificateValidation::Revoked { serial_hex: serial });
            }
        }
        Ok(CertificateValidation::Valid)
    }

    fn inspect(&self, certificate: &[u8]) -> Result<CertificateSummary, DomainError> {
        let cert = parse_certificate(certificate)?;
        let tbs = &cert.tbs_certificate;
        Ok(CertificateSummary {
            subject: tbs.subject.to_string(),
            issuer: tbs.issuer.to_string(),
            serial_hex: serial_hex(&tbs.serial_number),
            not_before_unix: time_to_unix(&tbs.validity.not_before),
            not_after_unix: time_to_unix(&tbs.validity.not_after),
        })
    }
}

/// True when the certificate names the issuer's subject as its issuer and
/// its signature verifies under the issuer's public key.
fn chains_to_issuer(cert: &Certificate, issuer: &Certificate) -> Result<bool, DomainError> {
    let named_issuer = cert
        .tbs_certificate
        .issuer
        .to_der()
        .map_err(malformed_cert)?;
    let issuer_subject = issuer
        .tbs_certificate
        .subject
        .to_der()
        .map_err(malformed_cert)?;
    if named_issuer != issuer_subject {
        return Ok(false);
    }
    if cert.signature_algorithm.oid != SHA256_WITH_RSA_ENCRYPTION {
        return Ok(false);
    }
    let message = cert.tbs_certificate.to_der().map_err(malformed_cert)?;
    let signature = cert.signature.as_bytes().ok_or_else(|| {
        DomainError::MalformedCertificate("signature bit string has unused bits".to_string())
    })?;
    verify_rsa_sha256(issuer, &message, signature)
}

/// Verifies a sha256WithRSAEncryption signature over `message` with the
/// public key certified by `issuer`. An issuer key that is not RSA counts
/// as a failed verification, not as an error.
fn verify_rsa_sha256(
    issuer: &Certificate,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, DomainError> {
    let spki_der = issuer
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(malformed_cert)?;
    let Ok(public_key) = RsaPublicKey::from_public_key_der(&spki_der) else {
        return Ok(false);
    };
    let digest = Sha256::digest(message);
    Ok(public_key
        .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, signature)
        .is_ok())
}

/// Runs the mandatory revocation checks against a supplied list and
/// returns the matched serial, if any.
///
/// The list must parse, be signed by the presented issuer, and still be
/// within its scheduled update period at the evaluation time; each failed
/// requirement is a distinct hard error. The caller has already confirmed
/// that the issuer itself is inside its own validity window, so the list's
/// signature is only ever trusted under a currently valid issuer.
fn revoked_serial(
    crl_bytes: &[u8],
    cert: &Certificate,
    issuer: &Certificate,
    unix_seconds: i64,
) -> Result<Option<String>, DomainError> {
    let crl = parse_crl(crl_bytes)?;

    let crl_issuer = crl.tbs_cert_list.issuer.to_der().map_err(malformed_crl)?;
    let issuer_subject = issuer
        .tbs_certificate
        .subject
        .to_der()
        .map_err(malformed_cert)?;
    if crl_issuer != issuer_subject {
        return Err(DomainError::UntrustedCrl);
    }
    if crl.signature_algorithm.oid != SHA256_WITH_RSA_ENCRYPTION {
        return Err(DomainError::UntrustedCrl);
    }
    let message = crl.tbs_cert_list.to_der().map_err(malformed_crl)?;
    let signature = crl.signature.as_bytes().ok_or(DomainError::UntrustedCrl)?;
    if !verify_rsa_sha256(issuer, &message, signature)? {
        return Err(DomainError::UntrustedCrl);
    }

    let next_update = crl
        .tbs_cert_list
        .next_update
        .ok_or_else(|| DomainError::MalformedCrl("missing the next-update time".to_string()))?;
    let next_update_unix = time_to_unix(&next_update);
    if unix_seconds > next_update_unix {
        return Err(DomainError::StaleCrl { next_update_unix });
    }

    let serial = &cert.tbs_certificate.serial_number;
    let revoked = crl
        .tbs_cert_list
        .revoked_certificates
        .as_deref()
        .unwrap_or(&[]);
    Ok(revoked
        .iter()
        .find(|entry| entry.serial_number == *serial)
        .map(|entry| serial_hex(&entry.serial_number)))
}

fn malformed_cert(err: der::Error) -> DomainError {
    DomainError::MalformedCertificate(err.to_string())
}

fn malformed_crl(err: der::Error) -> DomainError {
    DomainError::MalformedCrl(err.to_string())
}

#[cfg(test)]
mod tests {
    use domain::crypto::certificate::CertificateValidator;
    use domain::DomainError;

    use super::X509ChainValidator;

    #[test]
    fn unparseable_certificate_bytes_are_a_hard_error() {
        let err = X509ChainValidator::new()
            .validate(b"garbage", b"also garbage", None, 0)
            .unwrap_err();
        assert!(matches!(err, DomainError::MalformedCertificate(_)));
    }

    #[test]
    fn inspect_rejects_unparseable_bytes() {
        let err = X509ChainValidator::new().inspect(b"garbage").unwrap_err();
        assert!(matches!(err, DomainError::MalformedCertificate(_)));
    }
}
