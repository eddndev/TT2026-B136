use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, CurrentDocumentMetadata, DocumentMetadata,
    DocumentOverview, DocumentPage, DocumentQuery, DocumentVersionRef, EvidenceExport,
    MetadataPage, MetadataQuery, MetadataRevision, VersionPage, VersionQuery,
};
use application::{verification::VerificationReport, ApplicationError};
use domain::{
    audit::ChainVerification,
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion},
};

pub struct UnusedDocuments;

impl CaseDocumentWorkflow for UnusedDocuments {
    fn get_metadata(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        unreachable!()
    }
    fn replace_metadata(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
        _expected: MetadataRevision,
        _values: DocumentMetadata,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        unreachable!()
    }
    fn metadata_history(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
        _query: MetadataQuery,
    ) -> Result<MetadataPage, ApplicationError> {
        unreachable!()
    }
    fn upload_with_metadata(
        &self,
        _token: &str,
        _case: CaseId,
        _name: &str,
        _bytes: &[u8],
        _values: DocumentMetadata,
    ) -> Result<DocumentOverview, ApplicationError> {
        unreachable!()
    }

    fn append(
        &self,
        _token: &str,
        _case: CaseId,
        _id: DocumentId,
        _expected: DocumentVersion,
        _name: &str,
        _bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
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
    ) -> Result<DocumentOverview, ApplicationError> {
        Err(ApplicationError::Port("unused".to_string()))
    }

    fn upload(
        &self,
        _token: &str,
        _case_id: CaseId,
        _name: &str,
        _document: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
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
