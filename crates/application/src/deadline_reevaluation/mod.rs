//! Versioned commitments for deadline reevaluation authorship and observations.
//!
//! These codecs validate the byte contract and reference shapes. They do not
//! authenticate a caller, verify persisted evidence or perform reevaluation.

mod model;
mod observations;
mod submission;
mod validation;
mod wire;

pub use model::*;
pub use observations::{decode_observations, encode_observations};
pub use submission::{decode_tracked_submission, encode_tracked_submission};

/// Largest valid DLTX2 frame: a user correction with maximum UTF-8 text.
pub const MAX_TRACKED_SUBMISSION_BYTES: usize = 5502;
/// Largest valid DLOB1 frame: private profile, notification, calendar and parent.
pub const MAX_OBSERVATIONS_BYTES: usize = 446;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TrackingCodecError {
    #[error("invalid tracking encoding: {0}")]
    InvalidEncoding(&'static str),
    #[error("invalid tracking shape: {0}")]
    InvalidShape(&'static str),
    #[error("tracking frame exceeds its byte limit")]
    SizeLimit,
}

pub(crate) use validation::technical_cause as validate_technical_cause;
