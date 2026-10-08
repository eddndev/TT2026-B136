use super::{clock, date, day, hex, known, note, offset, unknown, Declared, MeasureValidity};

#[test]
fn canonical_validity_matches_fixed_known_and_unknown_vectors() {
    let start = known(Declared::second(day("2026-10-04"), 10, 20, 30, offset(-6)).unwrap());
    let value = MeasureValidity::new(
        start,
        note("Until order"),
        Some(known(date("2026-10-05", None))),
    )
    .unwrap();
    assert_eq!(
        hex(&value),
        concat!(
            "4d56414c31",                     // MVAL1
            "0307ea0a040a141e01ffffaba000",   // Second, -06:00, no unknown reason
            "0000000b556e74696c206f72646572", // Until order
            "010107ea0a050000"                // End present, Date, no offset or unknown reason
        )
    );
    let unknowns = MeasureValidity::new(
        unknown("No start"),
        note("By order"),
        Some(unknown("No end")),
    )
    .unwrap();
    assert_eq!(
        hex(&unknowns),
        concat!(
            "4d56414c31",
            "0001000000084e6f207374617274", // Unknown with reason: No start
            "000000084279206f72646572",     // By order
            "010001000000064e6f20656e64"    // End present, Unknown with reason: No end
        )
    );
    let minute = MeasureValidity::new(
        known(clock(false, "2026-10-04", 10, 20, None)),
        note("X"),
        None,
    )
    .unwrap();
    assert_eq!(
        hex(&minute),
        concat!(
            "4d56414c31",
            "0207ea0a040a140000", // Minute, no offset or reason
            "0000000158",
            "00" // Statement X, no explicit end
        )
    );
}

#[test]
fn canonical_validity_binds_precision_offset_components_reasons_statement_and_end() {
    let baseline = Declared::second(day("2026-10-04"), 10, 20, 30, offset(0)).unwrap();
    let original = MeasureValidity::new(known(baseline), note("By order"), None)
        .unwrap()
        .canonical_bytes();
    for changed in [
        date("2026-10-04", offset(0)),
        clock(false, "2026-10-04", 10, 20, offset(0)),
        Declared::second(day("2026-10-04"), 10, 20, 30, None).unwrap(),
        Declared::second(day("2026-10-04"), 4, 20, 30, offset(-6)).unwrap(),
        Declared::second(day("2027-10-04"), 10, 20, 30, offset(0)).unwrap(),
        Declared::second(day("2026-11-04"), 10, 20, 30, offset(0)).unwrap(),
        Declared::second(day("2026-10-05"), 10, 20, 30, offset(0)).unwrap(),
        Declared::second(day("2026-10-04"), 11, 20, 30, offset(0)).unwrap(),
        Declared::second(day("2026-10-04"), 10, 21, 30, offset(0)).unwrap(),
        Declared::second(day("2026-10-04"), 10, 20, 31, offset(0)).unwrap(),
    ] {
        let value = MeasureValidity::new(known(changed), note("By order"), None).unwrap();
        assert_ne!(value.canonical_bytes(), original);
        let end = MeasureValidity::new(
            unknown("Start omitted"),
            note("By order"),
            Some(known(changed)),
        )
        .unwrap();
        let previous = MeasureValidity::new(
            unknown("Start omitted"),
            note("By order"),
            Some(known(baseline)),
        )
        .unwrap();
        assert_ne!(end.canonical_bytes(), previous.canonical_bytes());
    }
    for value in [
        MeasureValidity::new(known(baseline), note("Other order"), None).unwrap(),
        MeasureValidity::new(known(baseline), note("By order"), Some(known(baseline))).unwrap(),
        MeasureValidity::new(
            known(baseline),
            note("By order"),
            Some(unknown("End omitted")),
        )
        .unwrap(),
        MeasureValidity::new(unknown("Start omitted"), note("By order"), None).unwrap(),
    ] {
        assert_ne!(value.canonical_bytes(), original);
    }
    let left = MeasureValidity::new(
        unknown("Illegible"),
        note("By order"),
        Some(unknown("Omitted")),
    )
    .unwrap();
    let changed_start = MeasureValidity::new(
        unknown("Unavailable"),
        note("By order"),
        Some(unknown("Omitted")),
    )
    .unwrap();
    let changed_end = MeasureValidity::new(
        unknown("Illegible"),
        note("By order"),
        Some(unknown("Unavailable")),
    )
    .unwrap();
    assert_ne!(left.canonical_bytes(), changed_start.canonical_bytes());
    assert_ne!(left.canonical_bytes(), changed_end.canonical_bytes());
}
