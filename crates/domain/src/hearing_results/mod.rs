//! Declared sessions and results preserve precision, history, and exact references.

mod canonical;
mod catalog;
mod identity;
mod references;
mod text;
mod time;
mod values;

pub use catalog::{
    HearingResultExtent, HearingResultOccurrence, HearingResultProvenanceKind, HearingResultStatus,
};
pub use identity::{
    HearingResultAgreementId, HearingResultId, HearingResultOperationId, HearingResultRevision,
};
pub use references::{
    HearingResultAgreement, HearingResultAttendee, HearingResultContinuationRef,
    HearingResultProvenance, HearingResultSupportRef,
};
pub use text::{
    HearingResultCapacity, HearingResultObservation, HearingResultReference, HearingResultText,
};
pub use time::{DeclaredHearingResultPrecision, DeclaredHearingResultTime};
pub use values::{
    HearingResultValues, HearingResultValuesInput, MAX_HEARING_RESULT_AGREEMENTS,
    MAX_HEARING_RESULT_ATTENDEES,
};

/// Smallest HRES1 representation, with a one-byte summary and date precision.
pub const MIN_HEARING_RESULT_CANONICAL_BYTES: usize = 26;
/// Largest HRES1 representation with all text encoded using four-byte scalars.
pub const MAX_HEARING_RESULT_CANONICAL_BYTES: usize = 146933;
