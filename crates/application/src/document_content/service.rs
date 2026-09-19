use super::{DocumentContent, DocumentContentWorkflow};
use crate::{
    document_integrity::{
        DocumentIntegrityFailure, DocumentIntegrityObservation, DocumentIntegrityObservationId,
        DocumentIntegrityStore,
    },
    documents::{
        CaseDocumentStore, DocumentAction, DocumentProcessor, DocumentRecord, VersionSelection,
    },
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{ArchiveEntry, DocumentVersionRef},
    identity::Permission,
};
use std::sync::Arc;

pub struct DocumentContentService {
    store: Arc<dyn CaseDocumentStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    clock: Arc<dyn Clock + Send + Sync>,
    incidents: Arc<dyn DocumentIntegrityStore>,
}

impl DocumentContentService {
    pub fn new(
        store: Arc<dyn CaseDocumentStore>,
        identity: Arc<dyn IdentityWorkflow>,
        processor: Arc<DocumentProcessor>,
        clock: Arc<dyn Clock + Send + Sync>,
        incidents: Arc<dyn DocumentIntegrityStore>,
    ) -> Self {
        Self {
            store,
            identity,
            processor,
            clock,
            incidents,
        }
    }

    fn principal(&self, token: &str) -> Result<Principal, ApplicationError> {
        let principal = self.identity.authenticate(token)?;
        if !principal.role.allows(Permission::ReadDocument) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }

    fn reauthenticate(&self, token: &str, expected: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    fn record_failure(
        &self,
        principal: &Principal,
        case_id: CaseId,
        record: DocumentRecord,
        failure: DocumentIntegrityFailure,
    ) -> Result<(), ApplicationError> {
        let observation = DocumentIntegrityObservation {
            observation_id: DocumentIntegrityObservationId::new(),
            requester: principal.id,
            case_id,
            record,
            failure,
            detected_at: self.clock.now(),
        };
        let receipt = self.incidents.record_rejection(&observation)?;
        let returned_at = self.clock.now();
        if receipt.observation_id != observation.observation_id
            || receipt.incident_id.as_uuid().is_nil()
            || receipt.recorded_at.offset() != time::UtcOffset::UTC
            || observation.detected_at.offset() != time::UtcOffset::UTC
            || returned_at.offset() != time::UtcOffset::UTC
            || receipt.recorded_at < observation.detected_at
            || receipt.recorded_at > returned_at
        {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "document integrity receipt does not match the observation".into(),
            ));
        }
        Ok(())
    }
}

impl DocumentContentWorkflow for DocumentContentService {
    fn content_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<DocumentContent, ApplicationError> {
        let principal = self.principal(token)?;
        let record = self.store.load(
            principal.id,
            case_id,
            reference.id,
            VersionSelection::Exact(reference.version),
            DocumentAction::ReadContent,
        )?;
        if record.id != reference.id || record.version != reference.version {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "content snapshot differs from the requested identity or version".into(),
            ));
        }
        ArchiveEntry::new(record.name.clone(), Vec::new()).map_err(|_| {
            ApplicationError::StoredDocumentInconsistent(
                "content snapshot has an invalid name".into(),
            )
        })?;
        let bytes = match self.processor.content_plaintext(&record) {
            Ok(bytes) => bytes,
            Err(ApplicationError::DocumentContentValidationFailed(failure)) => {
                let authority = self.reauthenticate(token, &principal);
                // Preserve the authorized observation even when the session was revoked.
                let persisted = self.record_failure(&principal, case_id, record, failure);
                authority?;
                persisted?;
                return Err(ApplicationError::DocumentContentValidationFailed(failure));
            }
            Err(error) => return Err(error),
        };
        self.reauthenticate(token, &principal)?;
        if let Err(error) = self.store.record_access(
            principal.id,
            case_id,
            &record,
            DocumentAction::ReadContent,
            self.clock.now(),
        ) {
            drop(bytes);
            if let ApplicationError::DocumentContentValidationFailed(failure) = error {
                self.record_failure(&principal, case_id, record, failure)?;
                return Err(ApplicationError::DocumentContentValidationFailed(failure));
            }
            return Err(error);
        }
        Ok(DocumentContent {
            case_id,
            reference,
            file_name: record.name,
            digest: record.digest,
            bytes,
        })
    }
}
