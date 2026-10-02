use super::CaseReportId;
use crate::ApplicationError;
use domain::crypto::{cipher::SealedPayload, keys::WrappedDek, Sha256Digest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportPayloadKind {
    Snapshot,
    Pdf,
    Csv,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseReportProtectionContext {
    pub report_id: CaseReportId,
    pub kind: CaseReportPayloadKind,
    pub plaintext_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCaseReportPayload {
    pub wrapped_dek: WrappedDek,
    pub payload: SealedPayload,
}
/// Protect report bytes with existing envelope cryptography. The versioned AAD
/// binds the complete context; open verifies its plaintext digest before return.
pub trait CaseReportProtector: Send + Sync {
    fn seal(
        &self,
        context: CaseReportProtectionContext,
        plaintext: &[u8],
    ) -> Result<ProtectedCaseReportPayload, ApplicationError>;
    fn open(
        &self,
        context: CaseReportProtectionContext,
        protected: &ProtectedCaseReportPayload,
    ) -> Result<Vec<u8>, ApplicationError>;
}
