//! Outbound port for reading the current time.
//!
//! Use cases that need a timestamp take it from this port instead of the
//! operating system, so tests can fix the instant they run at.

pub use time::OffsetDateTime;

/// Source of the current instant.
pub trait Clock {
    /// Returns the current instant.
    fn now(&self) -> OffsetDateTime;
}
