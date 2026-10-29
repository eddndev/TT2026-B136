use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    cases::CaseId,
    crypto::{CredentialCheck, DocumentHasher, DocumentVersionRef, Signature},
    document_signing::{DocumentConsent, SigningBindingStatement, Uuid},
};

use super::{
    evidence, PreparedDocumentSigning, SigningBindingReceipt, SigningReceipt,
    SigningWorkflowError as Error,
};
use crate::{credential_trust::CredentialTrustSnapshot, identity::Principal, ApplicationError};

pub(super) fn references(
    case: CaseId,
    document: DocumentVersionRef,
    operation: Uuid,
) -> Result<(), ApplicationError> {
    if case.as_uuid().is_nil() || document.id.as_uuid().is_nil() || operation.is_nil() {
        return Err(Error::Input.into());
    }
    Ok(())
}

pub(super) fn signature(value: &Signature) -> Result<(), ApplicationError> {
    if value.as_bytes().len() != 384 {
        return Err(Error::Input.into());
    }
    Ok(())
}

pub(super) fn successor(
    binding: &SigningBindingReceipt,
    trust: &CredentialTrustSnapshot,
) -> Result<(), ApplicationError> {
    let previous = &binding.trust;
    if trust.deployment_id != previous.deployment_id
        || trust.inspection.root_der != previous.inspection.root_der
        || trust.revision.get() < previous.revision.get()
        || (trust.revision == previous.revision && trust != previous)
        || (trust.revision != previous.revision
            && (trust.inspection.crl_number <= previous.inspection.crl_number
                || trust.inspection.crl_this_update < previous.inspection.crl_this_update))
    {
        return Err(Error::TrustChanged.into());
    }
    Ok(())
}

pub(super) fn verified(
    hasher: &dyn DocumentHasher,
    prepared: &PreparedDocumentSigning,
    check: &CredentialCheck,
    proof: &Signature,
    at: i64,
) -> Result<(), ApplicationError> {
    evidence::certificate(hasher, &check.certificate)?;
    evidence::trust(hasher, &prepared.trust)?;
    let summary = &check.certificate.summary;
    if check.certificate != prepared.binding.check.certificate
        || check.trust != prepared.trust.inspection
        || check.signature != *proof
        || check.statement_digest != hasher.hash_bytes(&prepared.consent.canonical_bytes())
        || check.checked_at != at
        || check.valid_from != summary.not_before_unix.max(check.trust.valid_from)
        || check.valid_until != summary.not_after_unix.min(check.trust.valid_until)
        || check.valid_from > check.valid_until
        || at < prepared.trust.published_at.unix_timestamp()
    {
        return Err(Error::Inconsistent.into());
    }
    evidence::check_time(check, at)
}

pub(super) fn exact_intent(
    receipt: &SigningReceipt,
    consent: &DocumentConsent,
    pdf: &Signature,
    proof: &Signature,
) -> Result<(), ApplicationError> {
    let individual = receipt
        .evidence
        .individual
        .as_ref()
        .ok_or(Error::Inconsistent)?;
    if receipt.consent != *consent
        || receipt.evidence.signature != pdf.as_bytes()
        || individual.consent_signature != proof.as_bytes()
    {
        return Err(Error::Conflict.into());
    }
    Ok(())
}

/// Canonical public PEM preserves exact DER while admitting no private material.
pub(super) fn pem(label: &str, bytes: &[u8]) -> Vec<u8> {
    let encoded = STANDARD.encode(bytes);
    let mut pem = format!("-----BEGIN {label}-----\n");
    for line in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
        pem.push('\n');
    }
    pem.push_str(&format!("-----END {label}-----\n"));
    pem.into_bytes()
}

fn decoded(value: &[u8], label: &str, maximum: usize) -> Result<Vec<u8>, ApplicationError> {
    if value.len() > maximum.saturating_mul(2) {
        return Err(Error::Inconsistent.into());
    }
    let text = std::str::from_utf8(value).map_err(|_| Error::Inconsistent)?;
    let body = text
        .strip_prefix(&format!("-----BEGIN {label}-----\n"))
        .and_then(|body| body.strip_suffix(&format!("-----END {label}-----\n")))
        .ok_or(Error::Inconsistent)?;
    let bytes = STANDARD
        .decode(body.lines().collect::<String>())
        .map_err(|_| Error::Inconsistent)?;
    if bytes.is_empty() || bytes.len() > maximum || pem(label, &bytes) != value {
        return Err(Error::Inconsistent.into());
    }
    Ok(bytes)
}

/// Validates stored public shape and exact identities, without checking current trust.
/// Cryptographic historical verification belongs to the document evidence processor.
pub fn validate_signing_receipt(
    hasher: &dyn DocumentHasher,
    receipt: &SigningReceipt,
    principal: &Principal,
    case: CaseId,
    document: DocumentVersionRef,
    operation: Uuid,
) -> Result<(), ApplicationError> {
    let consent = &receipt.consent;
    let evidence = &receipt.evidence;
    let individual = evidence.individual.as_ref().ok_or(Error::Inconsistent)?;
    if consent.account().id() != principal.id
        || consent.case() != case
        || consent.document() != document
        || consent.operation() != operation
        || individual.format_version != 1
        || individual.consent_statement != consent.canonical_bytes()
        || individual.binding_signature.len() != 384
        || individual.consent_signature.len() != 384
        || evidence.signature.len() != 384
        || evidence.timestamp_token.is_empty()
        || evidence.openssl_version.trim().is_empty()
        || evidence.tsa_chain_pem.as_ref().is_none_or(Vec::is_empty)
        || consent.pdf_signature_digest() != hasher.hash_bytes(&evidence.signature)
        || individual.accepted_at != receipt.accepted_at.unix_timestamp()
        || individual.binding_registered_at > individual.accepted_at
    {
        return Err(Error::Inconsistent.into());
    }
    if let Some(manifest) = &receipt.manifest {
        let parsed = manifest.check(operation, document, &consent.content_digest())?;
        let material = consent.material();
        let consistent = parsed.case_id == case.as_uuid().to_string()
            && parsed.deployment_id == material.deployment().to_string()
            && parsed.signers.iter().all(|signer| {
                signer.account_id == principal.id.as_uuid().to_string()
                    && signer.binding_id == material.binding().to_string()
                    && signer.binding_registered_at == individual.binding_registered_at
            });
        if !consistent {
            return Err(Error::Inconsistent.into());
        }
    }
    let binding = SigningBindingStatement::from_canonical_bytes(&individual.binding_statement)
        .map_err(|_| Error::Inconsistent)?;
    let registered = binding.material();
    let accepted = consent.material();
    if binding.account().id() != principal.id
        || binding.account().revision() > consent.account().revision()
        || binding.account().generation() > consent.account().generation()
        || registered.binding() != accepted.binding()
        || registered.deployment() != accepted.deployment()
        || registered.root() != accepted.root()
        || registered.leaf() != accepted.leaf()
        || registered.trust_revision() > accepted.trust_revision()
        || individual.binding_root_der.is_empty()
        || individual.binding_root_der.len() > 16384
        || individual.binding_crl_der.is_empty()
        || individual.binding_crl_der.len() > 1024 * 1024
    {
        return Err(Error::Inconsistent.into());
    }
    let certificate = decoded(&evidence.signer_certificate_pem, "CERTIFICATE", 16384)?;
    let root = decoded(&evidence.issuer_certificate_pem, "CERTIFICATE", 16384)?;
    let crl = decoded(&evidence.crl_pem, "X509 CRL", 1024 * 1024)?;
    if hasher.hash_bytes(&certificate) != accepted.leaf()
        || hasher.hash_bytes(&root) != accepted.root()
        || root != individual.binding_root_der
        || (registered.trust_revision() == accepted.trust_revision()
            && crl != individual.binding_crl_der)
    {
        return Err(Error::Inconsistent.into());
    }
    Ok(())
}
