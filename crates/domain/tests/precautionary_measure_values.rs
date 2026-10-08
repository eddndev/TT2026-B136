use domain::hearings::HearingNote;
use domain::judicial_calendars::CivilDate;
use domain::precautionary_measures::{MeasureKind, MeasureTime, MeasureValidity};
use domain::procedural_time::DeclaredProceduralTime as Declared;
use time::UtcOffset;

fn day(value: &str) -> CivilDate {
    value.parse().unwrap()
}

fn offset(hours: i8) -> Option<UtcOffset> {
    Some(UtcOffset::from_hms(hours, 0, 0).unwrap())
}

fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

fn known(value: Declared) -> MeasureTime {
    MeasureTime::new(value, None).unwrap()
}

fn unknown(reason: &str) -> MeasureTime {
    MeasureTime::new(Declared::unknown(), Some(note(reason))).unwrap()
}

fn date(value: &str, zone: Option<UtcOffset>) -> Declared {
    Declared::date(day(value), zone).unwrap()
}

fn clock(
    second_precision: bool,
    value: &str,
    hour: u8,
    minute: u8,
    zone: Option<UtcOffset>,
) -> Declared {
    if second_precision {
        Declared::second(day(value), hour, minute, 0, zone).unwrap()
    } else {
        Declared::minute(day(value), hour, minute, zone).unwrap()
    }
}

fn validity(start: Declared, end: Declared) -> Result<MeasureValidity, domain::DomainError> {
    MeasureValidity::new(known(start), note("As declared"), Some(known(end)))
}

fn hex(value: &MeasureValidity) -> String {
    value
        .canonical_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn measure_catalog_has_exactly_the_fourteen_declared_classes() {
    for (tag, (kind, name)) in [
        (MeasureKind::PeriodicAppearance, "periodic_appearance"),
        (MeasureKind::FinancialGuarantee, "financial_guarantee"),
        (MeasureKind::AssetSeizure, "asset_seizure"),
        (MeasureKind::AccountFreeze, "account_freeze"),
        (MeasureKind::TravelRestriction, "travel_restriction"),
        (MeasureKind::CustodyOrInstitution, "custody_or_institution"),
        (MeasureKind::PlaceRestriction, "place_restriction"),
        (MeasureKind::ContactRestriction, "contact_restriction"),
        (MeasureKind::HomeSeparation, "home_separation"),
        (
            MeasureKind::PublicOfficeSuspension,
            "public_office_suspension",
        ),
        (
            MeasureKind::ProfessionalSuspension,
            "professional_suspension",
        ),
        (MeasureKind::ElectronicMonitoring, "electronic_monitoring"),
        (MeasureKind::HomeConfinement, "home_confinement"),
        (MeasureKind::PretrialDetention, "pretrial_detention"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(kind.tag(), tag as u8);
        assert_eq!(kind.as_str(), name);
        assert_eq!(name.parse::<MeasureKind>().unwrap(), kind);
    }
    for invalid in [
        "",
        "other",
        "unknown",
        "protection_order",
        "initial",
        "PretrialDetention",
        " pretrial_detention",
        "pretrial_detention ",
    ] {
        assert!(invalid.parse::<MeasureKind>().is_err(), "{invalid}");
    }
}

#[test]
fn unknown_time_requires_a_reason_and_known_time_forbids_one() {
    assert!(MeasureTime::new(Declared::unknown(), None).is_err());
    let value = unknown(" Source omits the date ");
    assert_eq!(value.declared(), Declared::unknown());
    assert_eq!(
        value.unknown_reason().unwrap().as_str(),
        "Source omits the date"
    );
    for declared in [
        date("2026-10-04", None),
        date("2026-10-04", offset(0)),
        clock(false, "2026-10-04", 10, 20, None),
        clock(true, "2026-10-04", 10, 20, offset(-6)),
    ] {
        assert!(MeasureTime::new(declared, Some(note("Not stated"))).is_err());
        assert_eq!(known(declared).unknown_reason(), None);
    }
    assert!(HearingNote::new(" ").is_err());
    assert!(HearingNote::new(&"x".repeat(1001)).is_err());
    assert!(MeasureTime::new(Declared::unknown(), Some(note(&"x".repeat(1000)))).is_ok());
}

#[test]
fn time_preserves_declared_precision_components_and_optional_offset() {
    for zone in [
        None,
        offset(0),
        offset(-6),
        Some(UtcOffset::from_hms(5, 45, 0).unwrap()),
    ] {
        for declared in [
            date("2028-02-29", zone),
            Declared::minute(day("2028-02-29"), 10, 20, zone).unwrap(),
            Declared::second(day("2028-02-29"), 10, 20, 30, zone).unwrap(),
        ] {
            let captured = known(declared);
            assert_eq!(captured.declared(), declared);
            assert_eq!(captured.declared().precision(), declared.precision());
            assert_eq!(captured.declared().offset(), zone);
            assert_eq!(captured.declared().local_second(), declared.local_second());
            assert_eq!(captured.unknown_reason(), None);
        }
    }
}

#[test]
fn absent_end_and_explicitly_unknown_end_are_distinct_declarations() {
    let start = known(date("2026-10-04", None));
    let no_end = MeasureValidity::new(start.clone(), note(" Until further order "), None).unwrap();
    let unknown_end = MeasureValidity::new(
        start.clone(),
        note("Until further order"),
        Some(unknown("End illegible")),
    )
    .unwrap();
    assert_eq!(no_end.start(), &start);
    assert_eq!(no_end.statement().as_str(), "Until further order");
    assert!(no_end.end().is_none());
    assert_eq!(unknown_end.end().unwrap().declared(), Declared::unknown());
    assert_eq!(
        unknown_end
            .end()
            .unwrap()
            .unknown_reason()
            .unwrap()
            .as_str(),
        "End illegible"
    );
    assert_ne!(no_end.canonical_bytes(), unknown_end.canonical_bytes());
    assert!(MeasureValidity::new(
        unknown("Start omitted"),
        note("As declared"),
        Some(known(date("1900-01-01", None)))
    )
    .is_ok());
}

#[test]
fn civil_dates_compare_only_when_their_optional_offsets_match() {
    for zone in [None, offset(0), offset(-6)] {
        let start = date("2026-10-04", zone);
        assert!(validity(start, date("2026-10-03", zone)).is_err());
        assert!(validity(start, date("2026-10-04", zone)).is_ok());
        assert!(validity(start, date("2026-10-05", zone)).is_ok());
    }
    for (left, right) in [
        (None, offset(0)),
        (offset(0), None),
        (offset(-6), offset(0)),
    ] {
        let start = date("2026-10-04", left);
        let end = date("2026-10-03", right);
        let value = validity(start, end).unwrap();
        assert_eq!(value.start().declared(), start);
        assert_eq!(value.end().unwrap().declared(), end);
    }
}

#[test]
fn same_precision_explicit_instants_compare_in_utc_not_local_clock_order() {
    for second in [false, true] {
        let start = clock(second, "2026-10-04", 10, 0, offset(0));
        let before = clock(second, "2026-10-04", 11, 0, offset(2));
        let equal = clock(second, "2026-10-04", 4, 0, offset(-6));
        let after = clock(second, "2026-10-04", 9, 0, offset(-6));
        assert!(validity(start, before).is_err());
        assert!(validity(start, equal).is_ok());
        let value = validity(start, after).unwrap();
        assert_eq!(value.end().unwrap().declared(), after);
        let next_day = clock(second, "2026-10-05", 1, 0, offset(2));
        assert!(validity(next_day, clock(second, "2026-10-04", 22, 59, offset(0))).is_err());
        assert!(validity(next_day, clock(second, "2026-10-04", 23, 0, offset(0))).is_ok());
    }
    let start = Declared::second(day("2026-10-04"), 10, 20, 30, offset(0)).unwrap();
    let before = Declared::second(day("2026-10-04"), 10, 20, 29, offset(0)).unwrap();
    assert!(validity(start, before).is_err());
}

#[test]
fn missing_offsets_do_not_imply_a_shared_zone_even_with_the_same_precision() {
    for second in [false, true] {
        for (left, right) in [(None, None), (None, offset(0)), (offset(0), None)] {
            let start = clock(second, "2026-10-05", 10, 0, left);
            let end = clock(second, "2026-10-04", 9, 0, right);
            let value = validity(start, end).unwrap();
            assert_eq!(value.start().declared(), start);
            assert_eq!(value.end().unwrap().declared(), end);
        }
    }
}

#[test]
fn mixed_precision_retains_unresolved_end_without_filling_missing_components() {
    let later = [
        date("2026-10-05", offset(0)),
        clock(false, "2026-10-05", 10, 0, offset(0)),
        clock(true, "2026-10-05", 10, 0, offset(0)),
    ];
    let earlier = [
        date("2026-10-04", offset(0)),
        clock(false, "2026-10-04", 9, 0, offset(0)),
        clock(true, "2026-10-04", 9, 0, offset(0)),
    ];
    for start in later {
        for end in earlier {
            if start.precision() != end.precision() {
                let value = validity(start, end).unwrap();
                assert_eq!(value.start().declared(), start);
                assert_eq!(value.end().unwrap().declared(), end);
            }
        }
    }
}

#[path = "precautionary_measure_canonical.rs"]
mod canonical;
