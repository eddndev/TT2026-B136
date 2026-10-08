use super::*;

#[test]
fn unsupported_start_or_final_clock_future_capture_and_regression_reject_disclosure() {
    let saved = operation(40);
    let actor = reader(Role::Owner);
    let non_utc = now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for kind in READS {
        for start in [non_utc, year_zero] {
            let times = Arc::new(ReadClock(Mutex::new(vec![start])));
            assert!(read(
                &service(MockReads::new(), identity(&actor), times),
                &saved,
                kind
            )
            .is_err());
        }
        for times in [
            vec![now(), non_utc],
            vec![now(), year_zero],
            vec![now(), now() - time::Duration::nanoseconds(1)],
            vec![saved.capture.recorded_at - time::Duration::nanoseconds(1); 2],
        ] {
            let store = successful_store(&actor, &saved, saved.clone(), kind);
            assert!(read(
                &service(
                    store,
                    identity(&actor),
                    Arc::new(ReadClock(Mutex::new(times)))
                ),
                &saved,
                kind
            )
            .is_err());
        }
    }
}

#[test]
fn final_clock_can_reach_the_exact_capture_floor_without_losing_nanoseconds() {
    let saved = operation(40);
    let actor = reader(Role::Paralegal);
    let times = vec![
        saved.capture.recorded_at - time::Duration::nanoseconds(1),
        saved.capture.recorded_at,
    ];
    for kind in READS {
        let store = successful_store(&actor, &saved, saved.clone(), kind);
        let actual = read(
            &service(
                store,
                identity(&actor),
                Arc::new(ReadClock(Mutex::new(times.clone()))),
            ),
            &saved,
            kind,
        )
        .unwrap();
        assert_eq!(actual[0].capture.recorded_at, saved.capture.recorded_at);
    }
}
