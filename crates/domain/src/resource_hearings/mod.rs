//! Declared resource hearing values, separate from ordinary stage scheduling.

mod canonical;
mod catalog;
mod values;

pub use catalog::ResourceHearingKind;
pub use values::{
    ResourceHearingSchedulingBasis, ResourceHearingValues, ResourceHearingValuesInput,
    MAX_RESOURCE_HEARING_PARTICIPANTS,
};
