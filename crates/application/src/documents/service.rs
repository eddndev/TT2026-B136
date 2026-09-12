//! Offline document workflow; online routes use CaseDocumentService.

use std::sync::{Arc, Mutex};

use domain::audit::{AuditLog, ChainVerification};
use domain::clock::Clock;
use domain::crypto::{
    ArchiveWriter, AuthenticatedCipher, CertificateValidator, DocumentHasher, DocumentId,
    DocumentSigner, KeyManager, SignatureVerifier, TimestampService, TimestampVerifier,
};
use zeroize::Zeroizing;

use super::{
    DocumentProcessor, DocumentProcessorPorts, DocumentRecord, DocumentRepository, DocumentSummary,
    DocumentWorkflow, EvidenceExport, EvidenceMaterial,
};
use crate::{verification::VerificationReport, ApplicationError};

/// Adapters for standalone local use; document and audit files are independent.
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

pub struct LocalDocumentWorkflow {
    repository: Arc<dyn DocumentRepository>,
    processor: DocumentProcessor,
    clock: Box<dyn Clock + Send + Sync>,
    operation_lock: Mutex<()>,
    audit_log: Mutex<Box<dyn AuditLog + Send + Sync>>,
}

impl LocalDocumentWorkflow {
    pub fn new(
        ports: DocumentWorkflowPorts,
        material: EvidenceMaterial,
        kek: Zeroizing<Vec<u8>>,
    ) -> Result<Self, ApplicationError> {
        let processor = DocumentProcessor::new(
            DocumentProcessorPorts {
                hasher: ports.hasher,
                cipher: ports.cipher,
                keys: ports.keys,
                signer: ports.signer,
                timestamp_service: ports.timestamp_service,
                signature_verifier: ports.signature_verifier,
                certificate_validator: ports.certificate_validator,
                timestamp_verifier: ports.timestamp_verifier,
                archiver: ports.archiver,
            },
            material,
            kek,
        )?;
        Ok(Self {
            repository: ports.repository,
            clock: ports.clock,
            processor,
            operation_lock: Mutex::new(()),
            audit_log: Mutex::new(ports.audit_log),
        })
    }

    fn load(&self, id: DocumentId) -> Result<DocumentRecord, ApplicationError> {
        self.repository
            .find(id)?
            .ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))
    }

    fn append_audit(
        &self,
        actor: &str,
        action: &str,
        id: DocumentId,
    ) -> Result<(), ApplicationError> {
        self.audit_log
            .lock()
            .map_err(|_| ApplicationError::Port("audit lock poisoned".into()))?
            .append(actor, action, &format!("document:{id}"), self.clock.now())?;
        Ok(())
    }

    fn operation_guard(
        &self,
        actor: &str,
    ) -> Result<std::sync::MutexGuard<'_, ()>, ApplicationError> {
        if actor.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "actor must not be blank".into(),
            ));
        }
        self.operation_lock
            .lock()
            .map_err(|_| ApplicationError::Port("workflow lock poisoned".into()))
    }
}

impl DocumentWorkflow for LocalDocumentWorkflow {
    fn upload(
        &self,
        actor: &str,
        name: &str,
        document: &[u8],
    ) -> Result<DocumentSummary, ApplicationError> {
        let _guard = self.operation_guard(actor)?;
        let record = self.processor.prepare(name, document)?;
        self.repository.insert(record.clone())?;
        self.append_audit(actor, "document.uploaded", record.id)?;
        Ok(DocumentSummary::from(&record))
    }

    fn seal(&self, actor: &str, id: DocumentId) -> Result<DocumentSummary, ApplicationError> {
        let _guard = self.operation_guard(actor)?;
        let record = self.processor.seal(&self.load(id)?)?;
        self.repository.replace(record.clone())?;
        self.append_audit(actor, "document.sealed", id)?;
        Ok(DocumentSummary::from(&record))
    }

    fn verify(&self, actor: &str, id: DocumentId) -> Result<VerificationReport, ApplicationError> {
        let _guard = self.operation_guard(actor)?;
        let report = self
            .processor
            .verify(&self.load(id)?, self.clock.now().unix_timestamp())?;
        self.append_audit(actor, "document.verified", id)?;
        Ok(report)
    }

    fn export_evidence(
        &self,
        actor: &str,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        let _guard = self.operation_guard(actor)?;
        let export = self.processor.export_evidence(&self.load(id)?)?;
        self.append_audit(actor, "document.evidence_exported", id)?;
        Ok(export)
    }

    fn verify_audit(&self) -> Result<ChainVerification, ApplicationError> {
        let entries = self
            .audit_log
            .lock()
            .map_err(|_| ApplicationError::Port("audit lock poisoned".into()))?
            .load_all()?;
        self.processor.verify_audit(&entries)
    }
}
