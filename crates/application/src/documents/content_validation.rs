use domain::DomainError;
use zeroize::Zeroizing;

use super::{validation::DocumentValidationPorts, DocumentRecord};
use crate::{
    document_content::MAX_DOCUMENT_CONTENT_BYTES,
    document_integrity::DocumentIntegrityFailure,
    vault::{decrypt_with_ports, VaultReadLimits},
    ApplicationError,
};

pub(super) fn plaintext(
    record: &DocumentRecord,
    kek: &[u8],
    ports: &DocumentValidationPorts<'_>,
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    let limits = VaultReadLimits::aes256_gcm(MAX_DOCUMENT_CONTENT_BYTES)?;
    let expected_length = limits.inspect(&record.vault).map_err(classify)?;
    let bytes = Zeroizing::new(
        decrypt_with_ports(
            ports.cipher,
            ports.keys,
            kek,
            &record.vault,
            record.id,
            record.version,
        )
        .map_err(classify)?,
    );
    if bytes.len() != expected_length {
        return Err(ApplicationError::DocumentContentValidationFailed(
            DocumentIntegrityFailure::MalformedVault,
        ));
    }
    if ports.hasher.hash_bytes(&bytes) != record.digest {
        return Err(ApplicationError::DocumentContentValidationFailed(
            DocumentIntegrityFailure::DigestMismatch,
        ));
    }
    Ok(bytes)
}

fn classify(error: ApplicationError) -> ApplicationError {
    use ApplicationError as A;
    match error {
        A::StageSupportTooLarge => A::DocumentContentTooLarge,
        A::MalformedVaultFile(_) | A::Domain(DomainError::MalformedSealedPayload { .. }) => {
            A::DocumentContentValidationFailed(DocumentIntegrityFailure::MalformedVault)
        }
        A::Domain(DomainError::AuthenticationFailed) => {
            A::DocumentContentValidationFailed(DocumentIntegrityFailure::AuthenticationFailed)
        }
        other => other,
    }
}
