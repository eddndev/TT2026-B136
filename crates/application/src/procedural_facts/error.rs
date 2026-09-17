use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProceduralFactError {
    #[error("procedural fact not found")]
    NotFound,
    #[error("exact procedural fact source not found")]
    ReferenceNotFound,
    #[error("procedural fact reference is not permitted")]
    InvalidReference,
    #[error("procedural fact revision changed")]
    RevisionConflict,
    #[error("procedural fact revision counter is exhausted")]
    RevisionExhausted,
    #[error("procedural fact is already withdrawn")]
    AlreadyWithdrawn,
    #[error("procedural fact operation identifier is already used")]
    OperationConflict,
    #[error("procedural fact submission differs from its preparation")]
    SubmissionMismatch,
    #[error("procedural fact support changed; validate it again")]
    SupportChanged,
    #[error("procedural fact support exceeds its admission size limit")]
    SupportTooLarge,
    #[error("procedural fact support is not an admitted PDF or DOCX")]
    SupportFormatRejected,
    #[error("procedural fact support validation exceeded its budget")]
    SupportValidationLimit,
    #[error("procedural fact support digest differs from the selected reference")]
    SupportDigestMismatch,
    #[error("stored procedural fact is inconsistent: {0}")]
    StoredInconsistent(String),
}
