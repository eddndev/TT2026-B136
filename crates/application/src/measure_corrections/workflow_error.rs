use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeasureAdministrativeError {
    #[error("administrative measure operation not found")]
    NotFound,
    #[error("complete administrative measure history exceeds the validation budget")]
    IncompleteHistory,
    #[error("administrative measure operation conflicts with its original command")]
    OperationConflict,
    #[error("administrative measure instruction differs from its preparation")]
    SubmissionMismatch,
    #[error("administrative measure review differs from its preparation")]
    ReviewMismatch,
    #[error("administrative measure target is no longer the current head")]
    StaleHead,
    #[error("administrative measure target has known dependants")]
    KnownDependants,
    #[error("stored administrative measure operation is inconsistent: {0}")]
    StoredInconsistent(String),
}
