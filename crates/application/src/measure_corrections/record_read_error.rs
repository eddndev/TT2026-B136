use crate::ApplicationError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MeasureRecordReadError {
    #[error("measure record not found")]
    NotFound,
    #[error("complete measure record history exceeds the validation budget")]
    IncompleteHistory,
    #[error("stored measure record is inconsistent: {0}")]
    StoredInconsistent(String),
}

pub(super) fn inconsistent(message: impl Into<String>) -> ApplicationError {
    MeasureRecordReadError::StoredInconsistent(message.into()).into()
}
