//! Content admission for new case-scoped uploads, separate from historical reads.

use crate::ApplicationError;

pub const MAX_DOCUMENT_UPLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmittedDocumentFormat {
    Pdf,
    Docx,
    Txt,
    Jpeg,
    Png,
    Mp3,
    Wav,
    Mp4,
}

/// Stable user-facing categories without parser diagnostics or document bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DocumentUploadError {
    #[error("document format is not supported")]
    Unsupported,
    #[error("document format is invalid")]
    Invalid,
    #[error("document validation exceeded its resource limit")]
    Limit,
    #[error("document validator is unavailable")]
    Unavailable,
}

/// Validate complete content under a bounded admission policy. The filename and
/// caller-supplied media type never determine the admitted format. Implementations
/// must finish or reject before returning; no persistence transaction is held.
pub trait DocumentUploadAdmission: Send + Sync {
    fn validate(&self, bytes: &[u8]) -> Result<AdmittedDocumentFormat, ApplicationError>;
}
