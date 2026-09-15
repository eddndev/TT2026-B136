//! Preparation resolves one snapshot; each commit rechecks that exact record.

use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};

use super::{
    CaseDocumentService, CaseDocumentSummary, DocumentAction, DocumentSummary, EvidenceExport,
    VersionSelection,
};
use crate::{verification::VerificationReport, ApplicationError};

impl CaseDocumentService {
    pub(super) fn append_content(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        expected: DocumentVersion,
        name: &str,
        bytes: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let permission = DocumentAction::Append.permission();
        let actor = self.actor(token, permission)?;
        let current = self.store.load(
            actor,
            case,
            id,
            VersionSelection::Current,
            DocumentAction::Append,
        )?;
        if current.version != expected {
            return Err(ApplicationError::DocumentVersionConflict);
        }
        let next = expected
            .next()
            .map_err(|_| ApplicationError::DocumentVersionExhausted)?;
        let record = self.processor.prepare_version(id, next, name, bytes)?;
        self.reauthenticate(token, actor, permission)?;
        self.store
            .append(actor, case, expected, record.clone(), self.clock.now())?;
        Ok(CaseDocumentSummary {
            case_id: case,
            document: DocumentSummary::from(&record),
        })
    }

    pub(super) fn seal_selected(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        selection: VersionSelection,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Seal.permission())?;
        let record = self
            .store
            .load(actor, case, id, selection, DocumentAction::Seal)?;
        let sealed = self.processor.seal(&record)?;
        self.reauthenticate(token, actor, DocumentAction::Seal.permission())?;
        self.store
            .seal(actor, case, sealed.clone(), self.clock.now())?;
        Ok(CaseDocumentSummary {
            case_id: case,
            document: DocumentSummary::from(&sealed),
        })
    }

    pub(super) fn verify_selected(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        selection: VersionSelection,
    ) -> Result<VerificationReport, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Verify.permission())?;
        let record = self
            .store
            .load(actor, case, id, selection, DocumentAction::Verify)?;
        let report = self
            .processor
            .verify(&record, self.clock.now().unix_timestamp())?;
        self.reauthenticate(token, actor, DocumentAction::Verify.permission())?;
        self.store.record_access(
            actor,
            case,
            &record,
            DocumentAction::Verify,
            self.clock.now(),
        )?;
        Ok(report)
    }

    pub(super) fn export_selected(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        selection: VersionSelection,
    ) -> Result<EvidenceExport, ApplicationError> {
        let actor = self.actor(token, DocumentAction::Export.permission())?;
        let record = self
            .store
            .load(actor, case, id, selection, DocumentAction::Export)?;
        let export = self.processor.export_evidence(&record)?;
        self.reauthenticate(token, actor, DocumentAction::Export.permission())?;
        self.store.record_access(
            actor,
            case,
            &record,
            DocumentAction::Export,
            self.clock.now(),
        )?;
        Ok(export)
    }
}
