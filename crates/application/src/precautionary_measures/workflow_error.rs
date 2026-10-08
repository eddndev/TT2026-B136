use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeasureDecisionError {
    #[error("measure decision not found")]
    NotFound,
    #[error("complete measure history exceeds the validation budget")]
    IncompleteHistory,
    #[error("measure operation conflicts with its original command")]
    OperationConflict,
    #[error("measure instruction differs from its preparation")]
    SubmissionMismatch,
    #[error("measure review differs from its preparation")]
    ReviewMismatch,
    #[error("stored measure decision is inconsistent: {0}")]
    StoredInconsistent(String),
}
