//! Explicit civil-day calendars with immutable, locally declared source references.

mod canonical;
mod catalog;
mod civil;
mod identity;
mod read;
mod rules;
mod scope;
mod source;
mod text;
mod values;

pub use catalog::{
    JudicialCalendarClassification, JudicialCalendarJurisdiction, JudicialCalendarStatus,
};
pub use civil::{CivilDate, JudicialCalendarCoverage};
pub use identity::{JudicialCalendarId, JudicialCalendarOperationId, JudicialCalendarRevision};
pub use rules::{JudicialCalendarException, JudicialCalendarRule, JudicialCalendarWeekdayRule};
pub use scope::{JudicialCalendarScope, JudicialCalendarScopeInput};
pub use source::{JudicialCalendarSource, JudicialCalendarSourceInput};
pub use text::JudicialCalendarReason;
pub use values::{JudicialCalendarDay, JudicialCalendarDayOrigin, JudicialCalendarValues};

pub const MAX_JUDICIAL_CALENDAR_SOURCES: usize = 16;
pub const MAX_JUDICIAL_CALENDAR_EXCEPTIONS: usize = 64;

/// Smallest JCAL1, with one entity and seven unresolved one-character explanations.
pub const MIN_JUDICIAL_CALENDAR_CANONICAL_BYTES: usize = 99;
/// Maximum JCAL1 with all cardinalities and four-byte Unicode scalar fields.
pub const MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES: usize = 191910;
