use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrecautionaryHearingError {
    #[error("precautionary hearing not found")]
    NotFound,
    #[error("complete precautionary hearing history exceeds the validation budget")]
    IncompleteHistory,
    #[error("precautionary operation conflicts with its original command")]
    OperationConflict,
    #[error("precautionary submission differs from its preparation")]
    SubmissionMismatch,
    #[error("precautionary review differs from its preparation")]
    ReviewMismatch,
    #[error("stored precautionary hearing is inconsistent: {0}")]
    StoredInconsistent(String),
}
