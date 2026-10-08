//! Declared precautionary appointments, separate from decisions and measures.

mod canonical;
mod catalog;
mod identity;
mod references;
mod values;

pub use catalog::PrecautionaryHearingPurpose;
pub use identity::{
    MeasureId, MeasureRevision, PrecautionaryHearingId, PrecautionaryHearingOperationId,
    PrecautionaryHearingRevision,
};
pub use references::{PrecautionaryHearingSchedulingBasis, PrecautionaryMeasureRef};
pub use values::{
    PrecautionaryHearingValues, PrecautionaryHearingValuesInput,
    MAX_PRECAUTIONARY_HEARING_PARTICIPANTS, MAX_PRECAUTIONARY_REVIEW_TARGETS,
};
