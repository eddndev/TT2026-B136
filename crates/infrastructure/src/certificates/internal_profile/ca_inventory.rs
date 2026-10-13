//! Historical inventory of the certificate purposes already issued by the CA.

use der::Encode;
use domain::crypto::{certificate::CertificateRequestProfile, CredentialFailure, Sha256Digest};

use super::{ca_inspection::inspect_ca_certificate, material::digest, profile, Profile};

/// Inventory classification only; this is not an issuance or credential policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaInventoryPurpose {
    Partner,
    DocumentSigning,
    TimeStamping,
}

/// Authenticated public metadata without current validity or revocation admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaInventoryCertificateInspection {
    pub serial: String,
    pub fingerprint: Sha256Digest,
    pub spki_sha256: Sha256Digest,
    pub root_fingerprint: Sha256Digest,
    pub purpose: CaInventoryPurpose,
    pub subject: String,
    pub not_before: i64,
    pub not_after: i64,
}

/// Inspects the existing Partner, document-signing and timestamping issuances.
///
/// The root self-signature, exact leaf profile and issuer signature are checked.
/// The timestamping profile is the fixed `v3_tsa` profile in `pki/tsa.cnf`; its
/// presence in an inventory does not authorize issuance, Owner authentication,
/// document signing or portal revocation. Use `inspect_ca_certificate` for the
/// two profiles supported by public-request issuance and recovery.
pub fn inspect_ca_inventory_certificate(
    certificate: &[u8],
    root: &[u8],
) -> Result<CaInventoryCertificateInspection, CredentialFailure> {
    match inspect_ca_certificate(certificate, root) {
        Ok(found) => {
            return Ok(CaInventoryCertificateInspection {
                serial: found.serial,
                fingerprint: found.fingerprint,
                spki_sha256: found.spki_sha256,
                root_fingerprint: found.root_fingerprint,
                purpose: match found.purpose {
                    CertificateRequestProfile::Partner => CaInventoryPurpose::Partner,
                    CertificateRequestProfile::DocumentSigning => {
                        CaInventoryPurpose::DocumentSigning
                    }
                },
                subject: found.subject,
                not_before: found.not_before,
                not_after: found.not_after,
            });
        }
        Err(CredentialFailure::UnsupportedCertificate) => {}
        Err(error) => return Err(error),
    }
    let authority = profile::certificate(root, Profile::Root)?;
    let leaf = profile::certificate(certificate, Profile::TimeStamping)?;
    leaf.check_issuer(&authority)?;
    let public_key = leaf
        .certificate
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| CredentialFailure::MalformedCertificate)?;
    let inspected = leaf.inspection();
    Ok(CaInventoryCertificateInspection {
        serial: inspected.summary.serial_hex,
        fingerprint: inspected.fingerprint,
        spki_sha256: digest(&public_key),
        root_fingerprint: digest(&authority.der),
        purpose: CaInventoryPurpose::TimeStamping,
        subject: inspected.summary.subject,
        not_before: inspected.summary.not_before_unix,
        not_after: inspected.summary.not_after_unix,
    })
}
