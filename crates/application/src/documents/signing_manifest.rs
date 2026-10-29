//! The sealed evidence manifest of an individual signature and the stored
//! facts its export needs.
//!
//! The manifest lists every covered entry of the evidence package and a
//! time-stamp token covers its digest; `tsa-crl.pem` lets a reader check the
//! time-stamping certificate at that token's `genTime`. Rationale: the
//! sections "The evidence manifest" and "Generation inside the acceptance
//! operation" of `docs/adr/0085-in-browser-evidence-validation.md`.

use domain::crypto::Sha256Digest;
use domain::document_signing::Uuid;
use domain::DomainError;
use evidence_verify::{manifest, profile};

use crate::ApplicationError;

/// Largest stored manifest or manifest token, in bytes.
pub const MANIFEST_LIMIT: usize = 64 * 1024;

/// The exact manifest bytes, the time-stamp token over their SHA-256 and the
/// PEM CRL of the time-stamping certificate issuer captured with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningManifest {
    pub manifest: Vec<u8>,
    pub token: Vec<u8>,
    pub tsa_crl_pem: Vec<u8>,
}

impl SigningManifest {
    /// SHA-256 of the exact manifest bytes.
    pub fn digest(&self) -> Sha256Digest {
        Sha256Digest::from_array(evidence_verify::certificate::sha256(&self.manifest))
    }

    /// Parses the manifest and checks that it names `operation`, `document`,
    /// its version and `content` digest; the sizes stay within the stored
    /// limits.
    pub fn check(
        &self,
        operation: Uuid,
        document: domain::crypto::DocumentVersionRef,
        content: &Sha256Digest,
    ) -> Result<manifest::Manifest, ApplicationError> {
        if self.manifest.is_empty()
            || self.manifest.len() > MANIFEST_LIMIT
            || self.token.is_empty()
            || self.token.len() > MANIFEST_LIMIT
            || self.tsa_crl_pem.is_empty()
            || self.tsa_crl_pem.len() > TSA_CRL_PEM_LIMIT
        {
            return Err(inconsistent());
        }
        let parsed = manifest::parse(&self.manifest).map_err(|_| inconsistent())?;
        if parsed.operation_id != operation.to_string()
            || parsed.document_id != document.id.as_uuid().to_string()
            || parsed.document_version != u64::from(document.version.get())
            || parsed.document_sha256 != *content.as_bytes()
        {
            return Err(inconsistent());
        }
        Ok(parsed)
    }
}

/// Largest stored PEM CRL: the 1 MiB DER limit in base64 with its line
/// breaks and armor.
pub const TSA_CRL_PEM_LIMIT: usize = 1_500_000;

/// A stored manifest with the receipt facts the export writes into
/// `recibo-aceptacion.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestExport {
    pub operation: Uuid,
    pub accepted_at_seconds: i64,
    pub accepted_at_nanoseconds: u32,
    pub manifest: SigningManifest,
}

/// Rejects a document name `S` for which `S`, `S.sig` or `S.tsr` equals,
/// ignoring ASCII case, a fixed entry name of any evidence package profile.
pub fn reserve_document_name(name: &str) -> Result<(), ApplicationError> {
    if profile::is_reserved_document_name(name) {
        return Err(DomainError::InvalidArchiveEntryName {
            name: name.to_owned(),
        }
        .into());
    }
    Ok(())
}

fn inconsistent() -> ApplicationError {
    ApplicationError::StoredDocumentInconsistent("individual signing manifest is invalid".into())
}
