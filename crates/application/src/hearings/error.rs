use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HearingError {
    #[error("hearing not found")]
    NotFound,
    #[error("hearing revision changed")]
    RevisionConflict,
    #[error("hearing revision counter is exhausted")]
    RevisionExhausted,
    #[error("hearing case or stage context changed")]
    ContextConflict,
    #[error("hearing requires a complete penal profile and a registered stage")]
    ContextRequired,
    #[error("hearing kind is incompatible with the current stage")]
    StageIncompatible,
    #[error("new hearing participant reference is no longer current and active")]
    ParticipantChanged,
    #[error("hearing is already cancelled")]
    AlreadyCancelled,
    #[error("hearing kind cannot change")]
    ImmutableKind,
    #[error("hearing operation identifier is already used")]
    OperationConflict,
    #[error("hearing submission differs from its preparation")]
    SubmissionMismatch,
    #[error("hearing support changed; validate it again")]
    SupportChanged,
    #[error("hearing support exceeds its admission size limit")]
    SupportTooLarge,
    #[error("hearing support is not an admitted PDF or DOCX")]
    SupportFormatRejected,
    #[error("hearing support validation exceeded its budget")]
    SupportValidationLimit,
    #[error("hearing support digest differs from the selected reference")]
    SupportDigestMismatch,
    #[error("stored hearing is inconsistent: {0}")]
    StoredInconsistent(String),
}
