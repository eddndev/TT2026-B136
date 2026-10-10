//! Export of captured public registration and document consent.

use std::collections::HashSet;

use domain::{
    crypto::{ArchiveEntry, ArchiveWriter, DocumentHasher},
    document_signing::{DocumentConsent, SigningBindingStatement},
    DomainError,
};
use time::OffsetDateTime;

use crate::{documents::IndividualSigningEvidence, ApplicationError};

use super::{
    assemble, finish, EvidencePackage, EvidenceRequest, INSTRUCTIONS_ENTRY_NAME,
    ISSUER_CERTIFICATE_ENTRY_NAME, TSA_CHAIN_ENTRY_NAME,
};

const INSTRUCTIONS: &str = include_str!("INDIVIDUAL-INSTRUCCIONES.template.md");

/// Adds individual public proofs to the existing document evidence package.
///
/// The caller must validate historical cryptography and persisted case association
/// before export. This assembly checks supported structure and digest correspondence;
/// it does not establish current authority or an external trust anchor.
pub fn export_individual_with_ports<H, A>(
    hasher: &H,
    archiver: &A,
    request: &EvidenceRequest<'_>,
    individual: &IndividualSigningEvidence,
) -> Result<EvidencePackage, ApplicationError>
where
    H: DocumentHasher + ?Sized,
    A: ArchiveWriter + ?Sized,
{
    let (binding, consent) = statements(individual)?;
    let mut assembly = assemble(hasher, request)?;
    if consent.content_digest().to_hex() != assembly.digest_hex
        || consent.pdf_signature_digest() != hasher.hash_bytes(request.signature)
    {
        return Err(invalid());
    }
    let token_anchor = if request.tsa_chain_pem.is_some() {
        TSA_CHAIN_ENTRY_NAME
    } else {
        ISSUER_CERTIFICATE_ENTRY_NAME
    };
    assembly.instructions = INSTRUCTIONS
        .replace("{{DOCUMENTO}}", request.document_name)
        .replace("{{FIRMA}}", &format!("{}.sig", request.document_name))
        .replace("{{SELLO}}", &format!("{}.tsr", request.document_name))
        .replace("{{DIGEST_SHA256}}", &assembly.digest_hex)
        .replace("{{ANCLA_SELLO}}", token_anchor)
        .replace("{{OPENSSL_VERSION}}", request.openssl_version)
        .replace(
            "{{REGISTRO_AT}}",
            &individual.binding_registered_at.to_string(),
        )
        .replace("{{ACEPTACION_AT}}", &individual.accepted_at.to_string())
        .replace("{{CUENTA}}", &consent.account().id().to_string())
        .replace(
            "{{DESPLIEGUE}}",
            &consent.material().deployment().to_string(),
        )
        .replace("{{VINCULO}}", &consent.material().binding().to_string())
        .replace("{{OPERACION}}", &consent.operation().to_string())
        .replace("{{EXPEDIENTE}}", &consent.case().to_string())
        .replace("{{DOCUMENTO_ID}}", &consent.document().id.to_string())
        .replace("{{VERSION}}", &consent.document().version.to_string())
        .replace("{{ROOT_DIGEST}}", &consent.material().root().to_hex())
        .replace("{{LEAF_DIGEST}}", &consent.material().leaf().to_hex())
        .replace(
            "{{REGISTRO_TRUST}}",
            &binding.material().trust_revision().to_string(),
        )
        .replace(
            "{{FIRMA_TRUST}}",
            &consent.material().trust_revision().to_string(),
        )
        .replace(
            "{{REGISTRO_DIGEST}}",
            &hasher.hash_bytes(&individual.binding_statement).to_hex(),
        )
        .replace(
            "{{CONSENT_DIGEST}}",
            &hasher.hash_bytes(&individual.consent_statement).to_hex(),
        )
        .replace(
            "{{PDF_FIRMA_DIGEST}}",
            &consent.pdf_signature_digest().to_hex(),
        );
    for (name, content) in [
        ("document-consent.bin", &individual.consent_statement),
        ("document-consent.sig", &individual.consent_signature),
        ("certificate-binding.bin", &individual.binding_statement),
        ("certificate-binding.sig", &individual.binding_signature),
        ("binding-root.der", &individual.binding_root_der),
        ("binding-crl.der", &individual.binding_crl_der),
    ] {
        assembly
            .entries
            .push(ArchiveEntry::new(name, content.clone())?);
    }
    let metadata = serde_json::to_vec(individual).map_err(|_| invalid())?;
    assembly
        .entries
        .push(ArchiveEntry::new("individual-signing.json", metadata)?);
    let mut names = HashSet::new();
    names.insert(INSTRUCTIONS_ENTRY_NAME);
    if assembly
        .entries
        .iter()
        .any(|entry| !names.insert(entry.name()))
    {
        return Err(DomainError::ArchiveWriteFailure(
            "individual evidence contains duplicate entry names".into(),
        )
        .into());
    }
    finish(archiver, assembly)
}

fn statements(
    evidence: &IndividualSigningEvidence,
) -> Result<(SigningBindingStatement, DocumentConsent), ApplicationError> {
    if evidence.format_version != 1
        || evidence.binding_signature.len() != 384
        || evidence.consent_signature.len() != 384
        || evidence.binding_root_der.is_empty()
        || evidence.binding_root_der.len() > 16 * 1024
        || evidence.binding_crl_der.is_empty()
        || evidence.binding_crl_der.len() > 1024 * 1024
        || evidence.accepted_at < evidence.binding_registered_at
        || OffsetDateTime::from_unix_timestamp(evidence.binding_registered_at).is_err()
        || OffsetDateTime::from_unix_timestamp(evidence.accepted_at).is_err()
    {
        return Err(invalid());
    }
    Ok((
        SigningBindingStatement::from_canonical_bytes(&evidence.binding_statement)
            .map_err(|_| invalid())?,
        DocumentConsent::from_canonical_bytes(&evidence.consent_statement)
            .map_err(|_| invalid())?,
    ))
}

fn invalid() -> ApplicationError {
    ApplicationError::StoredDocumentInconsistent("individual export evidence is invalid".into())
}
