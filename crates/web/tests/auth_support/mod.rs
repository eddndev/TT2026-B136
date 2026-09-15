use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, DocumentPage, DocumentQuery, DocumentVersionRef,
    EvidenceExport, VersionPage, VersionQuery,
};
use application::{verification::VerificationReport, ApplicationError};
use domain::{
    audit::ChainVerification,
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion},
};

pub struct UnusedDocuments;

impl CaseDocumentWorkflow for UnusedDocuments {
    fn append(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
        _expected: DocumentVersion,
        _name: &str,
        _bytes: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        unreachable!()
    }
    fn history(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
        _query: VersionQuery,
    ) -> Result<VersionPage, ApplicationError> {
        unreachable!()
    }
    fn get_version(
        &self,
        _token: &str,
        _case: CaseId,
        _reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        unreachable!()
    }
    fn seal_version(
        &self,
        _token: &str,
        _case: CaseId,
        _reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        unreachable!()
    }
    fn verify_version(
        &self,
        _token: &str,
        _case: CaseId,
        _reference: DocumentVersionRef,
    ) -> Result<VerificationReport, ApplicationError> {
        unreachable!()
    }
    fn export_version(
        &self,
        _token: &str,
        _case: CaseId,
        _reference: DocumentVersionRef,
    ) -> Result<EvidenceExport, ApplicationError> {
        unreachable!()
    }

    fn list(
        &self,
        _token: &str,
        _case_id: CaseId,
        _query: DocumentQuery,
    ) -> Result<DocumentPage, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn get(
        &self,
        _token: &str,
        _case_id: CaseId,
        _id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn upload(
        &self,
        _token: &str,
        _case_id: CaseId,
        _name: &str,
        _document: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        if _token == "client-token" {
            return Err(ApplicationError::PermissionDenied);
        }
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn seal(
        &self,
        _token: &str,
        _case_id: CaseId,
        _id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn verify(
        &self,
        _token: &str,
        _case_id: CaseId,
        _id: DocumentId,
    ) -> Result<VerificationReport, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn export_evidence(
        &self,
        _token: &str,
        _case_id: CaseId,
        _id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn verify_audit(&self, _token: &str) -> Result<ChainVerification, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }
}
