//! Case-scoped document preparation followed by an authorized database commit.

use std::sync::Arc;

use domain::audit::ChainVerification;
use domain::cases::CaseId;
use domain::clock::Clock;
use domain::crypto::DocumentId;
use domain::identity::{Permission, UserId};

use super::{
    CaseDocumentStore, CaseDocumentSummary, CaseDocumentWorkflow, DocumentAction,
    DocumentProcessor, DocumentSummary, EvidenceExport,
};
use crate::{identity::IdentityWorkflow, verification::VerificationReport, ApplicationError};

pub struct CaseDocumentService {
    store: Arc<dyn CaseDocumentStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: DocumentProcessor,
    clock: Arc<dyn Clock + Send + Sync>,
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

    fn actor(&self, token: &str, permission: Permission) -> Result<UserId, ApplicationError> {
        let principal = self.identity.authenticate(token)?;
        if !principal.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal.id)
    }

    fn reauthenticate(
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
    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        bytes: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Upload.permission())?;
        self.store
            .check_access(actor, case_id, DocumentAction::Upload)?;
        let record = self.processor.prepare(name, bytes)?;
        self.reauthenticate(token, actor, DocumentAction::Upload.permission())?;
        self.store
            .insert(actor, case_id, record.clone(), self.clock.now())?;
        Ok(CaseDocumentSummary {
            case_id,
            document: DocumentSummary::from(&record),
        })
    }

    fn seal(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Seal.permission())?;
        let record = self.store.load(actor, case_id, id, DocumentAction::Seal)?;
        let sealed = self.processor.seal(&record)?;
        self.reauthenticate(token, actor, DocumentAction::Seal.permission())?;
        self.store
            .seal(actor, case_id, sealed.clone(), self.clock.now())?;
        Ok(CaseDocumentSummary {
            case_id,
            document: DocumentSummary::from(&sealed),
        })
    }

    fn verify(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<VerificationReport, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Verify.permission())?;
        let record = self
            .store
            .load(actor, case_id, id, DocumentAction::Verify)?;
        let report = self
            .processor
            .verify(&record, self.clock.now().unix_timestamp())?;
        self.reauthenticate(token, actor, DocumentAction::Verify.permission())?;
        self.store.record_access(
            actor,
            case_id,
            &record,
            DocumentAction::Verify,
            self.clock.now(),
        )?;
        Ok(report)
    }

    fn export_evidence(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Export.permission())?;
        let record = self
            .store
            .load(actor, case_id, id, DocumentAction::Export)?;
        let export = self.processor.export_evidence(&record)?;
        self.reauthenticate(token, actor, DocumentAction::Export.permission())?;
        self.store.record_access(
            actor,
            case_id,
            &record,
            DocumentAction::Export,
            self.clock.now(),
        )?;
        Ok(export)
    }

    fn verify_audit(&self, token: &str) -> Result<ChainVerification, ApplicationError> {
        let actor = self.actor(token, Permission::VerifyAudit)?;
        let entries = self.store.audit_entries(actor)?;
        self.processor.verify_audit(&entries)
    }
}
