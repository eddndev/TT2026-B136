use domain::hearing_results::{
    DeclaredHearingResultPrecision as Precision, DeclaredHearingResultTime as Declared,
};
use time::macros::{date, datetime, offset};
use time::{Duration, UtcOffset};

#[test]
fn dates_keep_day_and_offset_without_claiming_an_instant() {
    let value = Declared::date(date!(2024 - 02 - 29), offset!(-6)).unwrap();
    assert_eq!(value.precision(), Precision::Date);
    assert_eq!(value.local_date(), date!(2024 - 02 - 29));
    assert_eq!(value.offset(), offset!(-6));
    assert_eq!(value.instant_value(), None);
    assert_eq!(value.lower_bound(), datetime!(2024-02-29 06:00 UTC));
    assert_eq!(
        value.upper_bound(),
        datetime!(2024-03-01 06:00 UTC) - Duration::nanoseconds(1)
    );
}

#[test]
fn instants_preserve_seconds_and_offset_with_identical_bounds() {
    let stamp = datetime!(1900-01-01 12:34:56 +5:30);
    let value = Declared::instant(stamp).unwrap();
    assert_eq!(value.precision(), Precision::Instant);
    assert_eq!(value.local_date(), date!(1900 - 01 - 01));
    assert_eq!(value.offset(), offset!(+5:30));
    assert_eq!(value.instant_value(), Some(stamp));
    assert_eq!(value.lower_bound(), datetime!(1900-01-01 07:04:56 UTC));
    assert_eq!(value.lower_bound(), value.upper_bound());
    assert!(Declared::instant(stamp + Duration::nanoseconds(1)).is_err());
}

#[test]
fn minute_offsets_include_both_fourteen_hour_extremes() {
    for seconds in [-50400, -19800, -60, 0, 60, 19800, 50400] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        let value = Declared::date(date!(2026 - 01 - 01), offset).unwrap();
        assert_eq!(value.offset(), offset);
        assert!(Declared::instant(date!(2026 - 01 - 01).midnight().assume_offset(offset)).is_ok());
    }
    for seconds in [-50460, -50401, -1, 1, 50401, 50460] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        assert!(Declared::date(date!(2026 - 01 - 01), offset).is_err());
        assert!(Declared::instant(date!(2026 - 01 - 01).midnight().assume_offset(offset)).is_err());
    }
}

#[test]
fn local_and_utc_year_bounds_apply_to_the_entire_declared_day() {
    for day in [date!(0001 - 01 - 01), date!(9999 - 12 - 31)] {
        assert!(Declared::date(day, UtcOffset::UTC).is_ok());
    }
    for (day, offset) in [
        (date!(0001 - 01 - 01), offset!(+14)),
        (date!(9999 - 12 - 31), offset!(-14)),
        (date!(0000 - 01 - 01), UtcOffset::UTC),
        (date!(-0001 - 01 - 01), UtcOffset::UTC),
    ] {
        assert!(Declared::date(day, offset).is_err());
    }
    assert!(Declared::instant(datetime!(0001-01-01 00:00 UTC)).is_ok());
    assert!(Declared::instant(datetime!(9999-12-31 23:59:59 UTC)).is_ok());
    assert!(Declared::instant(datetime!(0001-01-01 00:00 +14)).is_err());
    assert!(Declared::instant(datetime!(9999-12-31 23:59:59 -14)).is_err());
    assert!(Declared::instant(datetime!(0000-12-31 23:59 UTC)).is_err());
}

#[test]
fn equality_retains_precision_and_original_offset() {
    let local = datetime!(2026-12-31 23:50 -14);
    let utc = local.to_offset(UtcOffset::UTC);
    assert_eq!(local, utc);
    assert_ne!(
        Declared::instant(local).unwrap(),
        Declared::instant(utc).unwrap()
    );
    let day = Declared::date(date!(2026 - 01 - 01), UtcOffset::UTC).unwrap();
    assert_ne!(
        day,
        Declared::date(date!(2026 - 01 - 01), offset!(+1)).unwrap()
    );
    assert_ne!(
        day,
        Declared::instant(datetime!(2026-01-01 00:00 UTC)).unwrap()
    );
    assert_eq!(
        day,
        Declared::date(date!(2026 - 01 - 01), UtcOffset::UTC).unwrap()
    );
}

#[test]
fn date_lower_bound_supports_future_checks_without_assigning_midnight() {
    let value = Declared::date(date!(2026 - 01 - 01), offset!(+14)).unwrap();
    assert_eq!(value.lower_bound(), datetime!(2025-12-31 10:00 UTC));
    assert!(value.lower_bound() > datetime!(2025-12-31 09:59:59 UTC));
    assert!(value.lower_bound() <= datetime!(2025-12-31 10:00 UTC));
    assert_eq!(value.instant_value(), None);
}
