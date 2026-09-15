//! Case-scoped document preparation followed by an authorized database commit.

use std::sync::Arc;

use domain::audit::ChainVerification;
use domain::cases::CaseId;
use domain::clock::Clock;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::identity::{Permission, UserId};

use super::{
    CaseDocumentStore, CaseDocumentSummary, CaseDocumentWorkflow, CurrentDocumentMetadata,
    DocumentAction, DocumentMetadata, DocumentOverview, DocumentPage, DocumentProcessor,
    DocumentQuery, EvidenceExport, MetadataPage, MetadataQuery, MetadataRevision, VersionPage,
    VersionQuery, VersionSelection,
};
use crate::{identity::IdentityWorkflow, verification::VerificationReport, ApplicationError};

pub struct CaseDocumentService {
    pub(super) store: Arc<dyn CaseDocumentStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) processor: DocumentProcessor,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
}

impl CaseDocumentService {
    pub fn new(
        store: Arc<dyn CaseDocumentStore>,
        identity: Arc<dyn IdentityWorkflow>,
        processor: DocumentProcessor,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            processor,
            clock,
        }
    }

    pub(super) fn actor(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<UserId, ApplicationError> {
        self.actor_permissions(token, &[permission])
    }

    pub(super) fn actor_permissions(
        &self,
        token: &str,
        permissions: &[Permission],
    ) -> Result<UserId, ApplicationError> {
        let principal = self.identity.authenticate(token)?;
        if permissions
            .iter()
            .any(|permission| !principal.role.allows(*permission))
        {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal.id)
    }

    pub(super) fn reauthenticate(
        &self,
        token: &str,
        expected: UserId,
        permission: Permission,
    ) -> Result<(), ApplicationError> {
        if self.actor(token, permission)? != expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
}

impl CaseDocumentWorkflow for CaseDocumentService {
    fn get_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store
            .get_metadata(actor, case_id, id, self.clock.now())
    }

    fn replace_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        metadata: DocumentMetadata,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        let actor = self.actor(token, Permission::ClassifyDocument)?;
        self.store
            .replace_metadata(actor, case_id, id, expected, metadata, self.clock.now())
    }

    fn metadata_history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        query: MetadataQuery,
    ) -> Result<MetadataPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store
            .metadata_history(actor, case_id, id, query, self.clock.now())
    }

    fn upload_with_metadata(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
        metadata: DocumentMetadata,
    ) -> Result<DocumentOverview, ApplicationError> {
        let permissions = [Permission::CreateDocument, Permission::ClassifyDocument];
        let actor = self.actor_permissions(token, &permissions)?;
        self.store
            .check_access(actor, case_id, DocumentAction::Upload)?;
        self.store
            .check_access(actor, case_id, DocumentAction::Classify)?;
        let record = self.processor.prepare(name, bytes)?;
        if self.actor_permissions(token, &permissions)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        self.store
            .insert_with_metadata(actor, case_id, record, metadata, self.clock.now())
    }

    fn append(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected_version: DocumentVersion,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
        self.append_content(token, case_id, id, expected_version, name, bytes)
    }

    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        query: VersionQuery,
    ) -> Result<VersionPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store
            .history(actor, case_id, id, query, self.clock.now())
    }

    fn get_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store.get(
            actor,
            case_id,
            reference.id,
            VersionSelection::Exact(reference.version),
            self.clock.now(),
        )
    }

    fn seal_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.seal_selected(
            token,
            case_id,
            reference.id,
            VersionSelection::Exact(reference.version),
        )
    }

    fn verify_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<VerificationReport, ApplicationError> {
        self.verify_selected(
            token,
            case_id,
            reference.id,
            VersionSelection::Exact(reference.version),
        )
    }

    fn export_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<EvidenceExport, ApplicationError> {
        self.export_selected(
            token,
            case_id,
            reference.id,
            VersionSelection::Exact(reference.version),
        )
    }

    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: DocumentQuery,
    ) -> Result<DocumentPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store.list(actor, case_id, query, self.clock.now())
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<DocumentOverview, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDocument)?;
        self.store
            .get_overview(actor, case_id, id, self.clock.now())
    }

    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Upload.permission())?;
        self.store
            .check_access(actor, case_id, DocumentAction::Upload)?;
        let record = self.processor.prepare(name, bytes)?;
        self.reauthenticate(token, actor, DocumentAction::Upload.permission())?;
        self.store.insert(actor, case_id, record, self.clock.now())
    }

    fn seal(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.seal_selected(token, case_id, id, VersionSelection::Only)
    }

    fn verify(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<VerificationReport, ApplicationError> {
        self.verify_selected(token, case_id, id, VersionSelection::Only)
    }

    fn export_evidence(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        self.export_selected(token, case_id, id, VersionSelection::Only)
    }

    fn verify_audit(&self, token: &str) -> Result<ChainVerification, ApplicationError> {
        let actor = self.actor(token, Permission::VerifyAudit)?;
        let entries = self.store.audit_entries(actor)?;
        self.processor.verify_audit(&entries)
    }
}
