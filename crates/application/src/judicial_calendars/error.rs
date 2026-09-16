use thiserror::Error;
#[derive(Debug, Error)]
pub enum JudicialCalendarError {
    #[error("judicial calendar not found")]
    NotFound,
    #[error("judicial calendar revision changed")]
    RevisionConflict,
    #[error("judicial calendar operation already exists")]
    OperationConflict,
    #[error("judicial calendar is retired")]
    Retired,
    #[error("judicial calendar revision counter is exhausted")]
    RevisionExhausted,
    #[error("judicial calendar scope cannot change")]
    ScopeChangeForbidden,
    #[error("judicial calendar submission differs from preparation")]
    SubmissionMismatch,
    #[error("stored judicial calendar is inconsistent: {0}")]
    StoredInconsistent(String),
}
