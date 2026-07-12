//! Clock adapter backed by the operating system.

use domain::clock::Clock;
use time::OffsetDateTime;

/// [`Clock`] that reads the operating system time in UTC.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Creates the adapter; it holds no state.
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_utc_and_does_not_go_backwards() {
        let clock = SystemClock::new();
        let first = clock.now();
        let second = clock.now();
        assert_eq!(first.offset(), time::UtcOffset::UTC);
        assert!(second >= first);
    }
}
