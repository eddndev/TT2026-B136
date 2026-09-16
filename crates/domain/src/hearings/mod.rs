//! Declared hearing appointments with immutable value references and exact times.

mod canonical;
mod catalog;
mod identity;
mod references;
mod text;
mod time;
mod values;

pub use catalog::{HearingKind, HearingModality, HearingStatus};
pub use identity::{HearingId, HearingOperationId, HearingRevision};
pub use references::{HearingConvictionBasis, HearingParticipantRef, HearingSupportRef};
pub use text::{HearingNote, HearingVenue};
pub use time::HearingTime;
pub use values::{HearingValues, HearingValuesInput, MAX_HEARING_PARTICIPANTS};

/// Largest HEAR1 representation with all fields at their UTF-8 byte bounds.
pub const MAX_HEARING_CANONICAL_BYTES: usize = 10726;
