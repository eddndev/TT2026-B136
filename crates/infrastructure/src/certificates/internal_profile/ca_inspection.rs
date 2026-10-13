//! Public CA evidence inspection without current credential admission.

use der::{DecodePem, Encode};
use domain::crypto::{certificate::CertificateRequestProfile, CredentialFailure, Sha256Digest};
use domain::DomainError;
use x509_cert::request::CertReq;

use super::{material::digest, profile, Profile};
use crate::certificates::request_material;

/// Public request identity and the separately selected authority fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaRequestInspection {
    /// Hash of the exact supplied PEM bytes, including surrounding whitespace.
    pub csr_sha256: Sha256Digest,
    /// Hash of the DER SubjectPublicKeyInfo authenticated by the request proof.
    pub spki_sha256: Sha256Digest,
    /// Hash of the selected root's DER certificate; the CSR does not bind it.
    pub root_fingerprint: Sha256Digest,
    pub subject: String,
}

/// Authenticated issuance metadata, independent of current validity or revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaCertificateInspection {
    /// Certificate serial in the existing uppercase hexadecimal representation.
    pub serial: String,
    /// Hash of the complete DER certificate, independent of PEM formatting.
    pub fingerprint: Sha256Digest,
    pub spki_sha256: Sha256Digest,
    pub root_fingerprint: Sha256Digest,
    pub purpose: CertificateRequestProfile,
    pub subject: String,
    /// Signed validity boundary in Unix seconds, not a current admission result.
    pub not_before: i64,
    /// Signed validity boundary in Unix seconds, not a current admission result.
    pub not_after: i64,
}

/// Inspect one bounded public PKCS#10 request and one strict internal root.
///
/// The request proof authenticates its subject and public key. The caller must
/// compare the returned root fingerprint with its configured or journaled pin:
/// a PKCS#10 request does not contain a cryptographic commitment to that root.
/// No private material is accepted and no authority state is changed.
pub fn inspect_ca_request(csr: &[u8], root: &[u8]) -> Result<CaRequestInspection, DomainError> {
    let public_key = request_material::inspect(csr)?;
    let authority = profile::certificate(root, Profile::Root).map_err(|_| {
        DomainError::CertificateAuthorityFailure(
            "CA inspection requires a strict self-signed internal root".into(),
        )
    })?;
    let request = CertReq::from_pem(csr.trim_ascii())
        .map_err(|_| request_material::failure("validated public request cannot be decoded"))?;
    Ok(CaRequestInspection {
        csr_sha256: digest(csr),
        spki_sha256: digest(&public_key),
        root_fingerprint: digest(&authority.der),
        subject: request.info.subject.to_string(),
    })
}

/// Inspect a directly issued Partner or document-signing certificate.
///
/// Strict profiles, the root's self-signature and the leaf's issuer/signature
/// are verified using the same mechanics as credential admission. There is no
/// clock or CRL input: the signed dates are returned for historical recovery,
/// without asserting present validity or non-revocation. The supplied root is
/// selected by the caller and must match the caller's configured authority pin.
pub fn inspect_ca_certificate(
    certificate: &[u8],
    root: &[u8],
) -> Result<CaCertificateInspection, CredentialFailure> {
    let authority = profile::certificate(root, Profile::Root)?;
    let (leaf, purpose) = match profile::certificate(certificate, Profile::Declaration) {
        Ok(leaf) => (leaf, CertificateRequestProfile::DocumentSigning),
        Err(CredentialFailure::UnsupportedCertificate) => (
            profile::certificate(certificate, Profile::Partner)?,
            CertificateRequestProfile::Partner,
        ),
        Err(error) => return Err(error),
    };
    leaf.check_issuer(&authority)?;
    let public_key = leaf
        .certificate
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| CredentialFailure::MalformedCertificate)?;
    let inspected = leaf.inspection();
    Ok(CaCertificateInspection {
        serial: inspected.summary.serial_hex,
        fingerprint: inspected.fingerprint,
        spki_sha256: digest(&public_key),
        root_fingerprint: digest(&authority.der),
        purpose,
        subject: inspected.summary.subject,
        not_before: inspected.summary.not_before_unix,
        not_after: inspected.summary.not_after_unix,
    })
}
