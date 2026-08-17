//! Application service for the complete local document workflow.

use std::sync::{Arc, Mutex};

use domain::audit::{verify_chain, AuditLog, ChainVerification};
use domain::clock::Clock;
use domain::crypto::keys::KEY_ENCRYPTION_KEY_LEN;
use domain::crypto::{
    ArchiveEntry, ArchiveWriter, AuthenticatedCipher, CertificateValidator, DocumentHasher,
    DocumentId, DocumentSigner, DocumentVersion, KeyManager, Signature, SignatureVerification,
    SignatureVerifier, TimestampService, TimestampVerification, TimestampVerifier,
};
use zeroize::Zeroizing;

use super::{
    DocumentRecord, DocumentRepository, DocumentSummary, DocumentWorkflow, EvidenceExport,
    SealedEvidence,
};
use crate::evidence::{export_with_ports, EvidenceRequest};
use crate::vault::{decrypt_with_ports, encrypt_with_ports};
use crate::verification::{
    verify_with_ports, TimestampEvidence, VerificationReport, VerifyDocumentRequest,
};
use crate::ApplicationError;

/// Outbound adapters required by [`LocalDocumentWorkflow`].
pub struct DocumentWorkflowPorts {
    pub repository: Arc<dyn DocumentRepository>,
    pub audit_log: Box<dyn AuditLog + Send + Sync>,
    pub clock: Box<dyn Clock + Send + Sync>,
    pub hasher: Box<dyn DocumentHasher + Send + Sync>,
    pub cipher: Box<dyn AuthenticatedCipher + Send + Sync>,
    pub keys: Box<dyn KeyManager + Send + Sync>,
    pub signer: Box<dyn DocumentSigner + Send + Sync>,
    pub timestamp_service: Box<dyn TimestampService + Send + Sync>,
    pub signature_verifier: Box<dyn SignatureVerifier + Send + Sync>,
    pub certificate_validator: Box<dyn CertificateValidator + Send + Sync>,
    pub timestamp_verifier: Box<dyn TimestampVerifier + Send + Sync>,
    pub archiver: Box<dyn ArchiveWriter + Send + Sync>,
}

struct RuntimePorts {
    repository: Arc<dyn DocumentRepository>,
    clock: Box<dyn Clock + Send + Sync>,
    hasher: Box<dyn DocumentHasher + Send + Sync>,
    cipher: Box<dyn AuthenticatedCipher + Send + Sync>,
    keys: Box<dyn KeyManager + Send + Sync>,
    signer: Box<dyn DocumentSigner + Send + Sync>,
    timestamp_service: Box<dyn TimestampService + Send + Sync>,
    signature_verifier: Box<dyn SignatureVerifier + Send + Sync>,
    certificate_validator: Box<dyn CertificateValidator + Send + Sync>,
    timestamp_verifier: Box<dyn TimestampVerifier + Send + Sync>,
    archiver: Box<dyn ArchiveWriter + Send + Sync>,
}

/// Public evidence captured at sealing time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceMaterial {
    pub signer_certificate_pem: Vec<u8>,
    pub issuer_certificate_pem: Vec<u8>,
    pub crl_pem: Vec<u8>,
    pub tsa_chain_pem: Option<Vec<u8>>,
    pub openssl_version: String,
}

/// Thread-safe implementation of the document workflow port.
pub struct LocalDocumentWorkflow {
    ports: RuntimePorts,
    material: EvidenceMaterial,
    kek: Zeroizing<Vec<u8>>,
    operation_lock: Mutex<()>,
    audit_log: Mutex<Box<dyn AuditLog + Send + Sync>>,
}

impl LocalDocumentWorkflow {
    /// Builds the workflow after validating runtime key and evidence material.
    pub fn new(
        ports: DocumentWorkflowPorts,
        material: EvidenceMaterial,
        kek: Zeroizing<Vec<u8>>,
    ) -> Result<Self, ApplicationError> {
        if kek.len() != KEY_ENCRYPTION_KEY_LEN {
            return Err(ApplicationError::InvalidConfiguration(format!(
                "document kek must be {KEY_ENCRYPTION_KEY_LEN} bytes, got {}",
                kek.len()
            )));
        }
        validate_material(&material)?;
        let DocumentWorkflowPorts {
            repository,
            audit_log,
            clock,
            hasher,
            cipher,
            keys,
            signer,
            timestamp_service,
            signature_verifier,
            certificate_validator,
            timestamp_verifier,
            archiver,
        } = ports;
        let ports = RuntimePorts {
            repository,
            clock,
            hasher,
            cipher,
            keys,
            signer,
            timestamp_service,
            signature_verifier,
            certificate_validator,
            timestamp_verifier,
            archiver,
        };
        Ok(Self {
            ports,
            material,
            kek,
            operation_lock: Mutex::new(()),
            audit_log: Mutex::new(audit_log),
        })
    }

    fn load(&self, id: DocumentId) -> Result<DocumentRecord, ApplicationError> {
        self.ports
            .repository
            .find(id)?
            .ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))
    }

    fn plaintext(&self, record: &DocumentRecord) -> Result<Vec<u8>, ApplicationError> {
        let plaintext = decrypt_with_ports(
            self.ports.cipher.as_ref(),
            self.ports.keys.as_ref(),
            &self.kek,
            &record.vault,
            record.id,
            record.version,
        )?;
        let digest = self.ports.hasher.hash_bytes(&plaintext);
        if digest != record.digest {
            return Err(ApplicationError::StoredDocumentInconsistent(format!(
                "digest mismatch for {}",
                record.id
            )));
        }
        Ok(plaintext)
    }

    fn append_audit(
        &self,
        actor: &str,
        action: &str,
        id: DocumentId,
    ) -> Result<(), ApplicationError> {
        let mut log = self
            .audit_log
            .lock()
            .map_err(|_| ApplicationError::Port("audit lock poisoned".to_string()))?;
        log.append(
            actor,
            action,
            &format!("document:{id}"),
            self.ports.clock.now(),
        )?;
        Ok(())
    }

    fn operation_guard(&self) -> Result<std::sync::MutexGuard<'_, ()>, ApplicationError> {
        self.operation_lock
            .lock()
            .map_err(|_| ApplicationError::Port("workflow lock poisoned".to_string()))
    }
}

impl DocumentWorkflow for LocalDocumentWorkflow {
    fn upload(
        &self,
        actor: &str,
        name: &str,
        document: &[u8],
    ) -> Result<DocumentSummary, ApplicationError> {
        validate_actor(actor)?;
        ArchiveEntry::new(name.to_string(), Vec::new())?;
        let _guard = self.operation_guard()?;
        let id = DocumentId::new();
        let version = DocumentVersion::initial();
        let digest = self.ports.hasher.hash_bytes(document);
        let vault = encrypt_with_ports(
            self.ports.cipher.as_ref(),
            self.ports.keys.as_ref(),
            &self.kek,
            document,
            id,
            version,
        )?;
        let record = DocumentRecord::pending(id, version, name.to_string(), digest, vault)?;
        self.ports.repository.insert(record.clone())?;
        self.append_audit(actor, "document.uploaded", id)?;
        Ok(DocumentSummary::from(&record))
    }

    fn seal(&self, actor: &str, id: DocumentId) -> Result<DocumentSummary, ApplicationError> {
        validate_actor(actor)?;
        let _guard = self.operation_guard()?;
        let mut record = self.load(id)?;
        if record.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(id.to_string()));
        }
        let plaintext = self.plaintext(&record)?;
        let signature = self.ports.signer.sign(&record.digest)?;
        ensure_signature_matches(
            self.ports.signature_verifier.as_ref(),
            &record.digest,
            &signature,
            &self.material.signer_certificate_pem,
        )?;
        let token = self.ports.timestamp_service.request(&record.digest)?;
        let anchor = self
            .material
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&self.material.issuer_certificate_pem);
        ensure_timestamp_matches(
            self.ports.timestamp_verifier.as_ref(),
            &token,
            &record.digest,
            anchor,
        )?;
        let evidence = SealedEvidence {
            signature: signature.into_bytes(),
            timestamp_token: token,
            signer_certificate_pem: self.material.signer_certificate_pem.clone(),
            issuer_certificate_pem: self.material.issuer_certificate_pem.clone(),
            crl_pem: self.material.crl_pem.clone(),
            tsa_chain_pem: self.material.tsa_chain_pem.clone(),
            openssl_version: self.material.openssl_version.clone(),
        };
        record.seal(evidence)?;
        self.ports.repository.replace(record.clone())?;
        self.append_audit(actor, "document.sealed", id)?;
        drop(plaintext);
        Ok(DocumentSummary::from(&record))
    }

    fn verify(&self, actor: &str, id: DocumentId) -> Result<VerificationReport, ApplicationError> {
        validate_actor(actor)?;
        let _guard = self.operation_guard()?;
        let record = self.load(id)?;
        let plaintext = self.plaintext(&record)?;
        let evidence = record
            .evidence
            .as_ref()
            .ok_or_else(|| ApplicationError::DocumentNotSealed(id.to_string()))?;
        let signature = Signature::from_bytes(evidence.signature.clone())?;
        let anchor = evidence
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&evidence.issuer_certificate_pem);
        let request = VerifyDocumentRequest {
            signature: &signature,
            signer_certificate_pem: &evidence.signer_certificate_pem,
            issuer_certificate_pem: &evidence.issuer_certificate_pem,
            crl_pem: Some(&evidence.crl_pem),
            timestamp: Some(TimestampEvidence {
                token: &evidence.timestamp_token,
                trust_anchor_pem: anchor,
            }),
            evaluation_unix: self.ports.clock.now().unix_timestamp(),
        };
        let report = verify_with_ports(
            self.ports.hasher.as_ref(),
            self.ports.signature_verifier.as_ref(),
            self.ports.certificate_validator.as_ref(),
            self.ports.timestamp_verifier.as_ref(),
            &mut plaintext.as_slice(),
            &request,
        )?;
        self.append_audit(actor, "document.verified", id)?;
        Ok(report)
    }

    fn export_evidence(
        &self,
        actor: &str,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        validate_actor(actor)?;
        let _guard = self.operation_guard()?;
        let record = self.load(id)?;
        let plaintext = self.plaintext(&record)?;
        let evidence = record
            .evidence
            .as_ref()
            .ok_or_else(|| ApplicationError::DocumentNotSealed(id.to_string()))?;
        let request = EvidenceRequest {
            document_name: &record.name,
            document: &plaintext,
            signature: &evidence.signature,
            timestamp_token: &evidence.timestamp_token,
            signer_certificate_pem: &evidence.signer_certificate_pem,
            issuer_certificate_pem: &evidence.issuer_certificate_pem,
            crl_pem: &evidence.crl_pem,
            tsa_chain_pem: evidence.tsa_chain_pem.as_deref(),
            openssl_version: &evidence.openssl_version,
        };
        let package = export_with_ports(
            self.ports.hasher.as_ref(),
            self.ports.archiver.as_ref(),
            &request,
        )?;
        self.append_audit(actor, "document.evidence_exported", id)?;
        Ok(EvidenceExport {
            archive: package.archive,
            file_name: format!("{}-evidence.zip", record.name),
            document_digest_hex: package.document_digest_hex,
        })
    }

    fn verify_audit(&self) -> Result<ChainVerification, ApplicationError> {
        let log = self
            .audit_log
            .lock()
            .map_err(|_| ApplicationError::Port("audit lock poisoned".to_string()))?;
        let entries = log.load_all()?;
        Ok(verify_chain(self.ports.hasher.as_ref(), &entries)?)
    }
}

fn validate_actor(actor: &str) -> Result<(), ApplicationError> {
    if actor.trim().is_empty() {
        return Err(ApplicationError::InvalidInput(
            "actor must not be blank".to_string(),
        ));
    }
    Ok(())
}

fn validate_material(material: &EvidenceMaterial) -> Result<(), ApplicationError> {
    if material.signer_certificate_pem.is_empty()
        || material.issuer_certificate_pem.is_empty()
        || material.crl_pem.is_empty()
        || material.openssl_version.trim().is_empty()
    {
        return Err(ApplicationError::InvalidConfiguration(
            "certificate, crl, and openssl evidence material must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn ensure_signature_matches(
    verifier: &(dyn SignatureVerifier + Send + Sync),
    digest: &domain::crypto::Sha256Digest,
    signature: &Signature,
    certificate: &[u8],
) -> Result<(), ApplicationError> {
    match verifier.verify(digest, signature, certificate)? {
        SignatureVerification::Valid => Ok(()),
        SignatureVerification::Invalid(cause) => Err(ApplicationError::InvalidConfiguration(
            format!("signing key does not match signer certificate: {cause}"),
        )),
    }
}

fn ensure_timestamp_matches(
    verifier: &(dyn TimestampVerifier + Send + Sync),
    token: &[u8],
    digest: &domain::crypto::Sha256Digest,
    anchor: &[u8],
) -> Result<(), ApplicationError> {
    match verifier.verify(token, digest, anchor)? {
        TimestampVerification::Valid { .. } => Ok(()),
        outcome => Err(ApplicationError::TimestampEvidenceRejected(format!(
            "{outcome:?}"
        ))),
    }
}
