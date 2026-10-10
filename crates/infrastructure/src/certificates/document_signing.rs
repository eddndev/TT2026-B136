//! Public proofs for individual signing, with a distinct binding and consent.
//!
//! The caller must select published trust and recheck account, binding and
//! document authority when committing. See docs/adr/0074-individual-document-signatures.md.

use application::credential_trust::CredentialTrustSnapshot;
use domain::{
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher, Signature,
    },
    document_signing::{DocumentConsent, SigningBindingMaterial, SigningBindingStatement},
};

use super::internal_profile::{Profile, Verifier};
use crate::RingSha256Hasher;

/// Rejections contain no submitted identities, credential bytes or parser text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DocumentSigningFailure {
    #[error("individual signing credential verification failed")]
    Credential(#[from] CredentialFailure),
    #[error("individual signing evidence differs from its captured public references")]
    MaterialMismatch,
}

/// Uses the internal signing-only leaf profile with mandatory CRL verification.
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalRsaDocumentSigningVerifier;

impl InternalRsaDocumentSigningVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Inspects bounded public material without establishing trust or validity.
    pub fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        Verifier::new(Profile::Declaration).inspect_certificate(certificate)
    }

    /// Verifies possession for the exact account-bound registration intention.
    pub fn verify_binding(
        &self,
        statement: &SigningBindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, DocumentSigningFailure> {
        let digest = RingSha256Hasher.hash_bytes(&statement.canonical_bytes());
        let check = Verifier::new(Profile::Declaration).verify(
            &digest,
            certificate,
            signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )?;
        ensure_material(statement.material(), &check, trust)?;
        Ok(check)
    }

    /// Verifies both the exact PDF digest and the distinct contextual consent.
    ///
    /// The returned check describes the consent signature. Its canonical bytes
    /// also bind the separately verified PDF signature and its content digest.
    pub fn verify_consent(
        &self,
        consent: &DocumentConsent,
        certificate: &[u8],
        pdf_signature: &Signature,
        consent_signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, DocumentSigningFailure> {
        if RingSha256Hasher.hash_bytes(pdf_signature.as_bytes()) != consent.pdf_signature_digest() {
            return Err(DocumentSigningFailure::MaterialMismatch);
        }
        let verifier = Verifier::new(Profile::Declaration);
        let pdf_check = verifier.verify(
            &consent.content_digest(),
            certificate,
            pdf_signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )?;
        ensure_material(consent.material(), &pdf_check, trust)?;
        let digest = RingSha256Hasher.hash_bytes(&consent.canonical_bytes());
        let check = verifier.verify(
            &digest,
            certificate,
            consent_signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )?;
        ensure_material(consent.material(), &check, trust)?;
        Ok(check)
    }
}

fn ensure_material(
    material: &SigningBindingMaterial,
    check: &CredentialCheck,
    trust: &CredentialTrustSnapshot,
) -> Result<(), DocumentSigningFailure> {
    if check.trust != trust.inspection
        || material.deployment() != trust.deployment_id
        || material.trust_revision() != trust.revision.get()
        || material.root() != check.trust.root_fingerprint
        || material.leaf() != check.certificate.fingerprint
    {
        return Err(DocumentSigningFailure::MaterialMismatch);
    }
    Ok(())
}

impl application::document_signing::DocumentSigningVerifier for InternalRsaDocumentSigningVerifier {
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        InternalRsaDocumentSigningVerifier::inspect_certificate(self, certificate)
    }

    fn verify_binding(
        &self,
        statement: &SigningBindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, application::document_signing::SigningWorkflowError> {
        InternalRsaDocumentSigningVerifier::verify_binding(
            self,
            statement,
            certificate,
            signature,
            trust,
            at,
        )
        .map_err(workflow_failure)
    }

    fn verify_consent(
        &self,
        consent: &DocumentConsent,
        certificate: &[u8],
        pdf_signature: &Signature,
        consent_signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, application::document_signing::SigningWorkflowError> {
        InternalRsaDocumentSigningVerifier::verify_consent(
            self,
            consent,
            certificate,
            pdf_signature,
            consent_signature,
            trust,
            at,
        )
        .map_err(workflow_failure)
    }
}

fn workflow_failure(
    error: DocumentSigningFailure,
) -> application::document_signing::SigningWorkflowError {
    use application::document_signing::SigningWorkflowError;
    match error {
        DocumentSigningFailure::Credential(failure) => SigningWorkflowError::Credential(failure),
        DocumentSigningFailure::MaterialMismatch => SigningWorkflowError::Inconsistent,
    }
}
