use domain::{
    judicial_calendars::CivilDate,
    procedural_time::{
        DeclaredProceduralPrecision as Precision, DeclaredProceduralTime as Declared,
    },
};
use time::{macros::datetime, UtcOffset};

fn date(value: &str) -> CivilDate {
    value.parse().unwrap()
}

#[test]
fn unknown_has_no_temporal_fields_or_instant() {
    let value = Declared::unknown();
    assert_eq!(value.precision(), Precision::Unknown);
    assert_eq!(value.local_date(), None);
    assert_eq!(value.local_hour(), None);
    assert_eq!(value.local_minute(), None);
    assert_eq!(value.local_second(), None);
    assert_eq!(value.offset(), None);
    assert_eq!(value.instant_value(), None);
}

#[test]
fn a_date_preserves_only_its_declared_date_and_optional_offset() {
    for offset in [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    ] {
        let day = date("2028-02-29");
        let value = Declared::date(day, offset).unwrap();
        assert_eq!(value.precision(), Precision::Date);
        assert_eq!(value.local_date(), Some(day));
        assert_eq!(value.offset(), offset);
        assert_eq!(value.local_hour(), None);
        assert_eq!(value.local_minute(), None);
        assert_eq!(value.local_second(), None);
        assert_eq!(value.instant_value(), None);
    }
}

#[test]
fn local_minute_does_not_invent_an_offset_second_or_instant() {
    let day = date("2026-09-16");
    let value = Declared::minute(day, 10, 34, None).unwrap();
    assert_eq!(value.precision(), Precision::Minute);
    assert_eq!(value.local_date(), Some(day));
    assert_eq!(value.local_hour(), Some(10));
    assert_eq!(value.local_minute(), Some(34));
    assert_eq!(value.local_second(), None);
    assert_eq!(value.offset(), None);
    assert_eq!(value.instant_value(), None);
}

#[test]
fn minute_with_explicit_offset_still_has_no_second_or_instant() {
    let offset = UtcOffset::from_hms(5, 45, 0).unwrap();
    let value = Declared::minute(date("2026-09-16"), 10, 34, Some(offset)).unwrap();
    assert_eq!(value.precision(), Precision::Minute);
    assert_eq!(value.local_date(), Some(date("2026-09-16")));
    assert_eq!(value.local_hour(), Some(10));
    assert_eq!(value.local_minute(), Some(34));
    assert_eq!(value.local_second(), None);
    assert_eq!(value.offset(), Some(offset));
    assert_eq!(value.instant_value(), None);
}

#[test]
fn local_second_without_offset_is_not_an_instant() {
    let day = date("2026-09-16");
    let value = Declared::second(day, 10, 34, 56, None).unwrap();
    assert_eq!(value.precision(), Precision::Second);
    assert_eq!(value.local_date(), Some(day));
    assert_eq!(value.local_hour(), Some(10));
    assert_eq!(value.local_minute(), Some(34));
    assert_eq!(value.local_second(), Some(56));
    assert_eq!(value.offset(), None);
    assert_eq!(value.instant_value(), None);
}

#[test]
fn explicit_second_returns_its_instant_and_keeps_the_original_local_fields() {
    let offset = UtcOffset::from_hms(14, 0, 0).unwrap();
    let value = Declared::second(date("2026-01-01"), 0, 0, 7, Some(offset)).unwrap();
    let instant = value.instant_value().unwrap();
    assert_eq!(value.precision(), Precision::Second);
    assert_eq!(value.local_date(), Some(date("2026-01-01")));
    assert_eq!(value.local_hour(), Some(0));
    assert_eq!(value.local_minute(), Some(0));
    assert_eq!(value.local_second(), Some(7));
    assert_eq!(instant, datetime!(2025-12-31 10:00:07 UTC));
    assert_eq!(instant.offset(), offset);
    assert_eq!(instant.nanosecond(), 0);
}

#[test]
fn equality_keeps_precision_and_distinguishes_absent_offset_from_declared_utc() {
    let day = date("2026-01-01");
    let values = [
        Declared::unknown(),
        Declared::date(day, None).unwrap(),
        Declared::date(day, Some(UtcOffset::UTC)).unwrap(),
        Declared::minute(day, 0, 0, None).unwrap(),
        Declared::minute(day, 0, 0, Some(UtcOffset::UTC)).unwrap(),
        Declared::second(day, 0, 0, 0, None).unwrap(),
        Declared::second(day, 0, 0, 0, Some(UtcOffset::UTC)).unwrap(),
    ];
    for (i, left) in values.iter().enumerate() {
        for (j, right) in values.iter().enumerate() {
            assert_eq!(left == right, i == j);
        }
    }
}

#[test]
fn equal_utc_instants_with_different_offsets_are_different_declarations() {
    let local = Declared::second(
        date("2026-01-01"),
        0,
        0,
        7,
        Some(UtcOffset::from_hms(14, 0, 0).unwrap()),
    )
    .unwrap();
    let utc = Declared::second(date("2025-12-31"), 10, 0, 7, Some(UtcOffset::UTC)).unwrap();
    assert_eq!(local.instant_value(), utc.instant_value());
    assert_ne!(local, utc);
    assert_ne!(
        Declared::minute(date("2026-01-01"), 0, 0, Some(UtcOffset::UTC)).unwrap(),
        Declared::minute(date("2026-01-01"), 0, 1, Some(UtcOffset::UTC)).unwrap(),
    );
}
