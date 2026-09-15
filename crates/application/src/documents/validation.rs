//! Validation of stored cryptographic bytes without a signing key or live TSA.

use domain::crypto::{
    ArchiveEntry, AuthenticatedCipher, CertificateValidation, CertificateValidator, DocumentHasher,
    KeyManager, Signature, SignatureVerification, SignatureVerifier, TimestampVerification,
    TimestampVerifier,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use zeroize::Zeroizing;

use super::DocumentRecord;
use crate::{vault::decrypt_with_ports, ApplicationError};

pub struct DocumentValidationPorts<'a> {
    pub cipher: &'a dyn AuthenticatedCipher,
    pub keys: &'a dyn KeyManager,
    pub hasher: &'a dyn DocumentHasher,
    pub signature_verifier: &'a dyn SignatureVerifier,
    pub certificate_validator: &'a dyn CertificateValidator,
    pub timestamp_verifier: &'a dyn TimestampVerifier,
}

/// Checks captured bytes and evaluates signer chain/CRL at the timestamp time.
/// TSA trust follows the supplied verifier, which may evaluate it at the current time.
/// This validates cryptographic consistency, not legal status or external trust.
pub fn validate_record_with_ports(
    record: &DocumentRecord,
    kek: &[u8],
    ports: &DocumentValidationPorts<'_>,
) -> Result<(), ApplicationError> {
    validated_plaintext_with_ports(record, kek, ports).map(|_| ())
}

pub(crate) fn validated_plaintext_with_ports(
    record: &DocumentRecord,
    kek: &[u8],
    ports: &DocumentValidationPorts<'_>,
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    ArchiveEntry::new(record.name.clone(), Vec::new())?;
    let plaintext = plaintext_with_ports(record, kek, ports)?;
    if let Some(evidence) = &record.evidence {
        if evidence.signer_certificate_pem.is_empty()
            || evidence.issuer_certificate_pem.is_empty()
            || evidence.crl_pem.is_empty()
            || evidence.openssl_version.trim().is_empty()
        {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "captured evidence material is incomplete".into(),
            ));
        }
        let signature = Signature::from_bytes(evidence.signature.clone())?;
        if ports.signature_verifier.verify(
            &record.digest,
            &signature,
            &evidence.signer_certificate_pem,
        )? != SignatureVerification::Valid
        {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "captured signature does not match the document".into(),
            ));
        }
        let anchor = evidence
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&evidence.issuer_certificate_pem);
        let generated_at = match ports.timestamp_verifier.verify(
            &evidence.timestamp_token,
            &record.digest,
            anchor,
        )? {
            TimestampVerification::Valid { generated_at } => generated_at,
            _ => {
                return Err(ApplicationError::StoredDocumentInconsistent(
                    "captured timestamp does not match the document".into(),
                ));
            }
        };
        let evaluation = OffsetDateTime::parse(&generated_at, &Rfc3339).map_err(|_| {
            ApplicationError::StoredDocumentInconsistent(
                "captured timestamp has an invalid generation time".into(),
            )
        })?;
        let certificate_status = ports.certificate_validator.validate(
            &evidence.signer_certificate_pem,
            &evidence.issuer_certificate_pem,
            Some(&evidence.crl_pem),
            evaluation.unix_timestamp(),
        )?;
        if certificate_status != CertificateValidation::Valid {
            return Err(ApplicationError::StoredDocumentInconsistent(format!(
                "captured signer material is invalid at timestamp time: {certificate_status}"
            )));
        }
    }
    Ok(plaintext)
}

pub(crate) fn plaintext_with_ports(
    record: &DocumentRecord,
    kek: &[u8],
    ports: &DocumentValidationPorts<'_>,
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    let plaintext = Zeroizing::new(decrypt_with_ports(
        ports.cipher,
        ports.keys,
        kek,
        &record.vault,
        record.id,
        record.version,
    )?);
    if ports.hasher.hash_bytes(&plaintext) != record.digest {
        return Err(ApplicationError::StoredDocumentInconsistent(format!(
            "digest mismatch for {}",
            record.id
        )));
    }
    Ok(plaintext)
}
