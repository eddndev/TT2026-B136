use domain::{
    judicial_calendars::CivilDate, procedural_time::DeclaredProceduralTime as Declared, DomainError,
};
use time::{macros::datetime, UtcOffset};

fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}
fn offset(seconds: i32) -> UtcOffset {
    UtcOffset::from_whole_seconds(seconds).unwrap()
}
fn invalid(value: Result<Declared, DomainError>) {
    assert_eq!(value, Err(DomainError::InvalidDeclaredProceduralTime));
}

#[test]
fn local_components_are_rejected_without_normalizing_or_truncating_them() {
    let day = date("2026-01-01");
    for zone in [None, Some(UtcOffset::UTC)] {
        for (hour, minute) in [(24, 0), (255, 0), (0, 60), (0, 255)] {
            invalid(Declared::minute(day, hour, minute, zone));
            invalid(Declared::second(day, hour, minute, 0, zone));
        }
        for second in [60, 255] {
            invalid(Declared::second(day, 23, 59, second, zone));
        }
    }
}

#[test]
fn valid_component_extremes_are_preserved_for_minute_and_second_precision() {
    let day = date("2028-02-29");
    for hour in [0, 23] {
        for minute in [0, 59] {
            let value = Declared::minute(day, hour, minute, None).unwrap();
            assert_eq!(value.local_hour(), Some(hour));
            assert_eq!(value.local_minute(), Some(minute));
            assert_eq!(value.local_second(), None);
            for second in [0, 59] {
                let value = Declared::second(day, hour, minute, second, None).unwrap();
                assert_eq!(value.local_hour(), Some(hour));
                assert_eq!(value.local_minute(), Some(minute));
                assert_eq!(value.local_second(), Some(second));
            }
        }
    }
}

#[test]
fn all_precisions_accept_minute_offsets_through_both_fourteen_hour_extremes() {
    let day = date("2026-01-01");
    for seconds in [-50400, -19800, -60, 0, 60, 20700, 50400] {
        let zone = Some(offset(seconds));
        for value in [
            Declared::date(day, zone),
            Declared::minute(day, 12, 0, zone),
            Declared::second(day, 12, 0, 0, zone),
        ] {
            assert_eq!(value.unwrap().offset(), zone);
        }
    }
    for seconds in [-86399, -50460, -50401, -1, 1, 50401, 50460, 86399] {
        let zone = Some(offset(seconds));
        invalid(Declared::date(day, zone));
        invalid(Declared::minute(day, 12, 0, zone));
        invalid(Declared::second(day, 12, 0, 0, zone));
    }
}

#[test]
fn both_civil_extremes_are_valid_without_inventing_a_zone_or_a_present_cutoff() {
    for day in [date("0001-01-01"), date("9999-12-31")] {
        for value in [
            Declared::date(day, None),
            Declared::minute(day, 0, 0, None),
            Declared::minute(day, 23, 59, None),
            Declared::second(day, 0, 0, 0, None),
            Declared::second(day, 23, 59, 59, None),
        ] {
            let value = value.unwrap();
            assert_eq!(value.local_date(), Some(day));
            assert_eq!(value.offset(), None);
            assert_eq!(value.instant_value(), None);
        }
    }
}

#[test]
fn a_declared_date_offset_must_keep_the_entire_day_within_utc_years() {
    let first = date("0001-01-01");
    let last = date("9999-12-31");
    for seconds in [60, 50400] {
        invalid(Declared::date(first, Some(offset(seconds))));
        invalid(Declared::date(last, Some(offset(-seconds))));
        assert!(Declared::date(first, Some(offset(-seconds))).is_ok());
        assert!(Declared::date(last, Some(offset(seconds))).is_ok());
    }
    assert!(Declared::date(first, Some(UtcOffset::UTC)).is_ok());
    assert!(Declared::date(last, Some(UtcOffset::UTC)).is_ok());
}

#[test]
fn a_declared_minute_offset_checks_utc_range_without_exposing_an_instant() {
    let first = date("0001-01-01");
    let last = date("9999-12-31");
    for (day, hour, minute, seconds) in [
        (first, 0, 0, 60),
        (first, 13, 59, 50400),
        (last, 23, 59, -60),
        (last, 10, 0, -50400),
    ] {
        invalid(Declared::minute(day, hour, minute, Some(offset(seconds))));
    }
    for (day, hour, minute, seconds) in [
        (first, 0, 1, 60),
        (first, 14, 0, 50400),
        (last, 23, 58, -60),
        (last, 9, 59, -50400),
        (first, 0, 0, 0),
        (last, 23, 59, 0),
    ] {
        let value = Declared::minute(day, hour, minute, Some(offset(seconds))).unwrap();
        assert_eq!(value.local_hour(), Some(hour));
        assert_eq!(value.local_minute(), Some(minute));
        assert_eq!(value.local_second(), None);
        assert_eq!(value.instant_value(), None);
    }
}

#[test]
fn exact_seconds_at_utc_boundaries_are_valid_and_crossing_seconds_are_rejected() {
    let first = date("0001-01-01");
    let last = date("9999-12-31");
    invalid(Declared::second(first, 13, 59, 59, Some(offset(50400))));
    invalid(Declared::second(last, 10, 0, 0, Some(offset(-50400))));
    let lower = Declared::second(first, 14, 0, 0, Some(offset(50400))).unwrap();
    let upper = Declared::second(last, 9, 59, 59, Some(offset(-50400))).unwrap();
    assert_eq!(
        lower.instant_value(),
        Some(datetime!(0001-01-01 00:00:00 UTC))
    );
    assert_eq!(
        upper.instant_value(),
        Some(datetime!(9999-12-31 23:59:59 UTC))
    );
    assert_eq!(lower.instant_value().unwrap().offset(), offset(50400));
    assert_eq!(upper.instant_value().unwrap().offset(), offset(-50400));
}
