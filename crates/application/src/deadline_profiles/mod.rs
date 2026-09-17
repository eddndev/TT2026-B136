//! Declared rule definitions with reproducible examples and explicit completion.
mod cutoff;
mod definition;
mod encoding;
mod model;
mod validation;
pub use cutoff::*;
pub use definition::*;
pub use encoding::{deadline_profile_definition_bytes, decode_deadline_profile_definition};
pub use model::*;

#[derive(Debug, thiserror::Error)]
pub enum DeadlineProfileError {
    #[error("deadline profile not found")]
    NotFound,
    #[error("deadline profile revision changed")]
    RevisionConflict,
    #[error("deadline profile operation already exists")]
    OperationConflict,
    #[error("deadline profile is retired")]
    Retired,
    #[error("deadline profile revision is exhausted")]
    RevisionExhausted,
    #[error("deadline profile scope cannot change")]
    ScopeChangeForbidden,
    #[error("deadline profile submission differs from preparation")]
    SubmissionMismatch,
    #[error("stored deadline profile is inconsistent: {0}")]
    StoredInconsistent(String),
    #[error("invalid deadline profile field: {0}")]
    Invalid(&'static str),
    #[error("deadline profile example does not reproduce: {0}")]
    ExampleMismatch(domain::typed_participants::Uuid),
    #[error("deadline profile needs a successful arithmetic example")]
    NoSuccessfulExample,
}

impl From<domain::deadline_profiles::DeadlineProfileValueError> for crate::ApplicationError {
    fn from(error: domain::deadline_profiles::DeadlineProfileValueError) -> Self {
        use domain::deadline_profiles::DeadlineProfileValueError as E;
        match error {
            E::RevisionExhausted => DeadlineProfileError::RevisionExhausted,
            E::InvalidRevision => DeadlineProfileError::Invalid("revision"),
            E::InvalidAlgorithm => DeadlineProfileError::Invalid("algorithm"),
            E::InvalidStatus => DeadlineProfileError::Invalid("status"),
        }
        .into()
    }
}

mod canonical;
mod catalog_model;
mod command;
mod port;
mod prepared;
mod query;
mod receipt;
pub use canonical::{
    deadline_profile_definition_digest, deadline_profile_submission_bytes,
    deadline_profile_submission_digest,
};
pub use catalog_model::*;
pub use domain::deadline_profiles::{
    DeadlineProfileAlgorithm, DeadlineProfileId, DeadlineProfileOperationId,
    DeadlineProfileRevision, DeadlineProfileStatus,
};
pub use port::*;
pub use prepared::*;
pub use query::*;
pub use receipt::{deadline_profile_history_receipt_matches, deadline_profile_receipt_matches};

mod catalog_validation;
mod service;
mod service_query;
pub use service::DeadlineProfileService;
