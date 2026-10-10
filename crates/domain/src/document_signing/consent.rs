use serde::Serialize;

use super::{canonical_prefix, require_exact, SignerAccount, SigningBindingMaterial, SigningError};
use crate::{
    cases::CaseId,
    crypto::{DocumentVersionRef, Sha256Digest},
};
use uuid::Uuid;

/// Exact document and detached-signature references for an external consent proof.
/// The supplied digests must be recomputed and verified by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DocumentConsent {
    account: SignerAccount,
    material: SigningBindingMaterial,
    operation: Uuid,
    case: CaseId,
    document: DocumentVersionRef,
    content_digest: Sha256Digest,
    pdf_signature_digest: Sha256Digest,
}

impl DocumentConsent {
    pub fn new(
        account: SignerAccount,
        material: SigningBindingMaterial,
        operation: Uuid,
        case: CaseId,
        document: DocumentVersionRef,
        content_digest: Sha256Digest,
        pdf_signature_digest: Sha256Digest,
    ) -> Result<Self, SigningError> {
        if operation.is_nil() || case.as_uuid().is_nil() || document.id.as_uuid().is_nil() {
            return Err(SigningError::InvalidIdentity);
        }
        Ok(Self {
            account,
            material,
            operation,
            case,
            document,
            content_digest,
            pdf_signature_digest,
        })
    }

    pub const fn account(&self) -> &SignerAccount {
        &self.account
    }

    pub const fn material(&self) -> &SigningBindingMaterial {
        &self.material
    }

    pub const fn operation(&self) -> Uuid {
        self.operation
    }

    pub const fn case(&self) -> CaseId {
        self.case
    }

    pub const fn document(&self) -> DocumentVersionRef {
        self.document
    }

    pub const fn content_digest(&self) -> Sha256Digest {
        self.content_digest
    }

    pub const fn pdf_signature_digest(&self) -> Sha256Digest {
        self.pdf_signature_digest
    }

    /// Encodes consent purpose 1 and format version 1 independently of registration.
    pub fn canonical_bytes(&self) -> [u8; 258] {
        let mut bytes = [0; 258];
        bytes[..142].copy_from_slice(&canonical_prefix(
            b"DOCSIGN1",
            &self.account,
            &self.material,
        ));
        bytes[142..158].copy_from_slice(self.operation.as_bytes());
        bytes[158..174].copy_from_slice(self.case.as_uuid().as_bytes());
        bytes[174..190].copy_from_slice(self.document.id.as_uuid().as_bytes());
        bytes[190..194].copy_from_slice(&self.document.version.get().to_be_bytes());
        bytes[194..226].copy_from_slice(self.content_digest.as_bytes());
        bytes[226..258].copy_from_slice(self.pdf_signature_digest.as_bytes());
        bytes
    }

    /// Rejects changed context or purpose without claiming a cryptographic check.
    pub fn require_exact_bytes(&self, supplied: &[u8]) -> Result<(), SigningError> {
        require_exact(supplied, &self.canonical_bytes())
    }
}
