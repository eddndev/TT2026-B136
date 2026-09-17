use domain::hearings::HearingTime;
use time::macros::{datetime, offset};
use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

#[test]
fn accepts_declared_past_and_future_without_a_clock_dependency() {
    for value in [
        datetime!(1900-01-01 10:00 -6),
        datetime!(9999-12-31 23:59:59 UTC),
    ] {
        let hearing = HearingTime::new(value).unwrap();
        assert_eq!(hearing.value(), value);
        assert_eq!(hearing.value().offset(), value.offset());
        assert_eq!(hearing.utc().offset(), UtcOffset::UTC);
        assert_eq!(hearing.utc().unix_timestamp(), value.unix_timestamp());
    }
}

#[test]
fn rejects_fractional_seconds_and_offsets_with_seconds() {
    assert!(HearingTime::new(datetime!(2026-09-15 00:00:00.000_000_001 UTC)).is_err());
    assert!(HearingTime::new(datetime!(2026-09-15 00:00:00.999_999_999 UTC)).is_err());
    for seconds in [-50401, -61, -1, 1, 61, 50401, 86400 - 1] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        assert!(HearingTime::new(datetime!(2026-09-15 00:00 UTC).to_offset(offset)).is_err());
    }
    for offset in [offset!(+14:01), offset!(-14:01), offset!(+15), offset!(-15)] {
        assert!(HearingTime::new(datetime!(2026-09-15 00:00 UTC).to_offset(offset)).is_err());
    }
}

#[test]
fn accepts_every_minute_offset_in_the_closed_fourteen_hour_interval() {
    for minute in -840..=840 {
        let offset = UtcOffset::from_whole_seconds(minute * 60).unwrap();
        let value = datetime!(2026-09-15 00:00 UTC).to_offset(offset);
        assert_eq!(HearingTime::new(value).unwrap().value().offset(), offset);
    }
}

#[test]
fn both_local_and_utc_years_must_be_between_one_and_9999() {
    let first = Date::from_calendar_date(1, Month::January, 1)
        .unwrap()
        .midnight();
    let last = Date::from_calendar_date(9999, Month::December, 31)
        .unwrap()
        .with_time(Time::from_hms(23, 59, 59).unwrap());
    assert!(HearingTime::new(first.assume_utc()).is_ok());
    assert!(HearingTime::new(last.assume_utc()).is_ok());
    assert!(HearingTime::new(first.assume_offset(offset!(+14))).is_err());
    assert!(HearingTime::new(last.assume_offset(offset!(-14))).is_err());
    assert!(HearingTime::new(first.assume_utc().to_offset(offset!(-14))).is_err());
    assert!(HearingTime::new(first.assume_offset(offset!(-14))).is_ok());
    assert!(HearingTime::new(last.assume_offset(offset!(+14))).is_ok());
    assert!(HearingTime::new(datetime!(0000-12-31 23:00 UTC)).is_err());
    assert!(HearingTime::new(datetime!(-0001-12-31 23:00 UTC)).is_err());
}

#[test]
fn utc_crossing_year_boundary_is_valid_inside_the_allowed_range() {
    let value = HearingTime::new(datetime!(2026-01-01 00:10 +14)).unwrap();
    assert_eq!(value.utc(), datetime!(2025-12-31 10:10 UTC));
    let value = HearingTime::new(datetime!(2026-12-31 23:50 -14)).unwrap();
    assert_eq!(value.utc(), datetime!(2027-01-01 13:50 UTC));
}

#[test]
fn equality_preserves_the_declared_offset_even_for_the_same_instant() {
    let local = HearingTime::new(datetime!(2026-09-15 09:00 -6)).unwrap();
    let utc = HearingTime::new(datetime!(2026-09-15 15:00 UTC)).unwrap();
    assert_eq!(local.utc(), utc.utc());
    assert_ne!(local, utc);
    assert_eq!(local, HearingTime::new(local.value()).unwrap());
    assert_ne!(
        utc,
        HearingTime::new(OffsetDateTime::from_unix_timestamp(0).unwrap()).unwrap()
    );
}
