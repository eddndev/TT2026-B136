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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test double frozen at a fixed instant.
    struct FrozenClock(OffsetDateTime);

    impl Clock for FrozenClock {
        fn now(&self) -> OffsetDateTime {
            self.0
        }
    }

    #[test]
    fn port_is_object_safe_and_returns_the_configured_instant() {
        let instant = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
        let clock: &dyn Clock = &FrozenClock(instant);
        assert_eq!(clock.now(), instant);
    }
}
