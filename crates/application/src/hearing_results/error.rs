use thiserror::Error;
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HearingResultError {
    #[error("hearing result not found")]
    NotFound,
    #[error("exact hearing result source not found")]
    ReferenceNotFound,
    #[error("hearing result revision changed")]
    RevisionConflict,
    #[error("hearing result revision counter is exhausted")]
    RevisionExhausted,
    #[error("hearing result is already withdrawn")]
    AlreadyWithdrawn,
    #[error("hearing result operation identifier is already used")]
    OperationConflict,
    #[error("hearing result submission differs from its preparation")]
    SubmissionMismatch,
    #[error("declared hearing result time is in the future")]
    FutureTime,
    #[error("hearing result reference is not permitted")]
    InvalidReference,
    #[error("hearing result support changed; validate it again")]
    SupportChanged,
    #[error("hearing result support exceeds its admission size limit")]
    SupportTooLarge,
    #[error("hearing result support is not an admitted PDF or DOCX")]
    SupportFormatRejected,
    #[error("hearing result support validation exceeded its budget")]
    SupportValidationLimit,
    #[error("hearing result support digest differs from the selected reference")]
    SupportDigestMismatch,
    #[error("stored hearing result is inconsistent: {0}")]
    StoredInconsistent(String),
}
