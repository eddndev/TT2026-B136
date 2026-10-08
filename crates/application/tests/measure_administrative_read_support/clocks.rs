use super::*;
use std::sync::{Arc, Mutex};

struct SequenceClock(Mutex<Vec<OffsetDateTime>>);
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let mut values = self.0.lock().unwrap();
        if values.len() == 1 {
            values[0]
        } else {
            values.remove(0)
        }
    }
}

#[test]
fn reads_reject_future_receipts_and_non_utc_unsupported_or_regressing_observations() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    let non_utc = now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for kind in READS {
        for values in [
            vec![original.capture.recorded_at - Duration::nanoseconds(1); 2],
            vec![now(), now() - Duration::nanoseconds(1)],
            vec![now(), non_utc],
            vec![now(), year_zero],
        ] {
            let store = successful_store(&actor, &original, original.clone(), kind);
            let clock = Arc::new(SequenceClock(Mutex::new(values)));
            assert!(read(
                &service_with_clock(store, identity(&actor), clock),
                kind,
                &original
            )
            .is_err());
        }
        for initial in [non_utc, year_zero] {
            assert!(read(
                &service_with_clock(
                    MockReads::new(),
                    identity(&actor),
                    Arc::new(FixedClock(initial))
                ),
                kind,
                &original
            )
            .is_err());
        }
    }
}

#[test]
fn final_observation_can_reach_the_exact_original_capture_time_and_preserve_nanoseconds() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    assert_ne!(original.capture.recorded_at.nanosecond(), 0);
    for kind in READS {
        let values = vec![
            original.capture.recorded_at - Duration::nanoseconds(1),
            original.capture.recorded_at,
        ];
        let store = successful_store(&actor, &original, original.clone(), kind);
        let clock = Arc::new(SequenceClock(Mutex::new(values)));
        assert_eq!(
            read(
                &service_with_clock(store, identity(&actor), clock),
                kind,
                &original
            )
            .unwrap(),
            vec![original.clone()]
        );
    }
}
