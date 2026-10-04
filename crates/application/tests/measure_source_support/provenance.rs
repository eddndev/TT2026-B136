use crate::measure_source_support::*;
use domain::clock::OffsetDateTime;
use time::{Date, Month, UtcOffset};

#[test]
fn every_retained_source_requires_supported_utc_provenance() {
    let invalid_times = [
        OffsetDateTime::UNIX_EPOCH.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        Date::from_calendar_date(-1, Month::December, 31)
            .unwrap()
            .midnight()
            .assume_utc(),
    ];
    for location in 0..4 {
        for at in invalid_times {
            let mut fixture = provenance_fixture(location);
            *provenance(&mut fixture, location).0 = at;
            assert!(
                fixture.resolve().is_err(),
                "invalid source clock at location {location}"
            );
        }
    }
}

#[test]
fn every_retained_source_requires_clean_actor_text() {
    for location in 0..4 {
        for email in [
            "",
            " leading@example.test",
            "trailing@example.test ",
            "a\nb",
            "a\0b",
            "a\x7fb",
        ] {
            let mut fixture = provenance_fixture(location);
            provenance(&mut fixture, location).1.email = email.into();
            assert!(
                fixture.resolve().is_err(),
                "invalid source actor at location {location}"
            );
        }
    }
}

#[test]
fn supported_source_times_and_clean_identity_text_need_no_capture_clock_or_email_parser() {
    for location in 0..4 {
        for year in [1, 9999] {
            let mut fixture = provenance_fixture(location);
            let (at, actor) = provenance(&mut fixture, location);
            *at = Date::from_calendar_date(year, Month::January, 1)
                .unwrap()
                .midnight()
                .assume_utc();
            actor.email = "historical identity".into();
            assert!(
                fixture.resolve().is_ok(),
                "supported source provenance at location {location}"
            );
        }
    }
}
