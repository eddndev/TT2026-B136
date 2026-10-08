use super::*;
use std::sync::{Arc, Mutex};

fn recorded_at(row: &MeasureRecordDetail) -> OffsetDateTime {
    match &row.record {
        OwnedMeasureRecord::Judicial(value) => value.recorded_at(),
        OwnedMeasureRecord::Administrative { capture, .. } => capture.recorded_at,
    }
}
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
fn reads_reject_future_records_and_invalid_or_regressing_service_clocks() {
    let original = mixed(10).pop().unwrap();
    let actor = reader(Role::Paralegal);
    let non_utc = now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for kind in READS {
        for values in [
            vec![recorded_at(&original) - Duration::nanoseconds(1); 2],
            vec![now(), now() - Duration::nanoseconds(1)],
            vec![now(), non_utc],
            vec![now(), year_zero],
        ] {
            let store = successful_store(&actor, &original, original.clone(), kind);
            assert!(read(
                &service_with_clock(
                    store,
                    identity(&actor),
                    Arc::new(SequenceClock(Mutex::new(values)))
                ),
                kind,
                &original
            )
            .is_err());
        }
        for invalid in [non_utc, year_zero] {
            assert!(read(
                &service_with_clock(
                    MockReads::new(),
                    identity(&actor),
                    Arc::new(FixedClock(invalid))
                ),
                kind,
                &original
            )
            .is_err());
        }
    }
}

#[test]
fn exact_capture_nanoseconds_are_preserved_when_the_final_clock_reaches_the_source_floor() {
    let original = mixed(10).pop().unwrap();
    let actor = reader(Role::Paralegal);
    let at = recorded_at(&original);
    assert_ne!(at.nanosecond(), 0);
    for kind in READS {
        let store = successful_store(&actor, &original, original.clone(), kind);
        let clock = Arc::new(SequenceClock(Mutex::new(vec![
            at - Duration::nanoseconds(1),
            at,
        ])));
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
