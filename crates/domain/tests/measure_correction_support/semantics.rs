use domain::precautionary_measures::{
    MeasureCorrectionValues, MeasureKind, MeasureSupervision, MeasureTime, MeasureValidity,
    MeasureValues,
};
use domain::procedural_time::DeclaredProceduralTime as Declared;
use domain::DomainError;
use time::UtcOffset;

use crate::support::*;

fn assert_reason(result: Result<MeasureValues, DomainError>, expected: &str) {
    match result {
        Err(DomainError::InvalidPrecautionaryMeasure(reason)) => assert_eq!(reason, expected),
        other => panic!("expected correction error {expected}, got {other:?}"),
    }
}

fn declarations() -> Vec<MeasureTime> {
    vec![
        unknown("Time not legible"),
        known(Declared::date(day(), None).unwrap()),
        known(Declared::minute(day(), 1, 2, None).unwrap()),
        known(Declared::second(day(), 1, 2, 3, Some(UtcOffset::UTC)).unwrap()),
    ]
}

#[test]
fn unknown_supervision_remains_unknown_with_the_corrected_reason() {
    let mut source = input();
    source.supervision = MeasureSupervision::Unknown {
        reason: note("Not stated"),
    };
    let original = MeasureValues::new(source);
    let before = original.canonical_bytes();
    let changes = MeasureCorrectionValues::new(
        original.conditions().clone(),
        original.validity().clone(),
        note("Name illegible in declaration"),
    );
    let corrected = original.correct_record(&changes).unwrap();
    assert_eq!(corrected.subject(), original.subject());
    assert_eq!(corrected.kind(), original.kind());
    assert_eq!(
        corrected.supervision(),
        &MeasureSupervision::Unknown {
            reason: changes.supervision_text().clone(),
        }
    );
    assert_eq!(original.canonical_bytes(), before);
}

#[test]
fn each_of_the_three_permitted_fields_can_change_independently() {
    let original = MeasureValues::new(input());
    let baseline = correction(&original);
    let changed_validity = MeasureValidity::new(
        original.validity().start().clone(),
        note("Corrected duration statement"),
        None,
    )
    .unwrap();
    for changes in [
        MeasureCorrectionValues::new(
            note("Corrected condition"),
            baseline.validity().clone(),
            baseline.supervision_text().clone(),
        ),
        MeasureCorrectionValues::new(
            baseline.conditions().clone(),
            changed_validity,
            baseline.supervision_text().clone(),
        ),
        MeasureCorrectionValues::new(
            baseline.conditions().clone(),
            baseline.validity().clone(),
            note("Corrected supervisor statement"),
        ),
    ] {
        let corrected = original.correct_record(&changes).unwrap();
        assert_eq!(corrected.conditions(), changes.conditions());
        assert_eq!(corrected.validity(), changes.validity());
        assert_eq!(corrected.subject(), original.subject());
        assert_eq!(corrected.kind(), original.kind());
        assert_eq!(
            corrected.supervision(),
            &MeasureSupervision::Known {
                participant: participant(),
                statement: changes.supervision_text().clone(),
            }
        );
    }
}

#[test]
fn normalized_identical_values_are_rejected_for_both_supervision_variants() {
    for supervision in [
        input().supervision,
        MeasureSupervision::Unknown {
            reason: note("Unknown"),
        },
    ] {
        let mut source = input();
        source.supervision = supervision;
        let original = MeasureValues::new(source);
        let baseline = correction(&original);
        let unchanged = MeasureCorrectionValues::new(
            note(&format!("  {}  ", baseline.conditions().as_str())),
            baseline.validity().clone(),
            note(&format!("  {}  ", baseline.supervision_text().as_str())),
        );
        let before = original.clone();
        assert_reason(original.correct_record(&unchanged), "correction_unchanged");
        assert_eq!(original, before);
    }
}

#[test]
fn adding_or_removing_an_end_is_rejected_even_when_other_text_changes() {
    for end in [
        unknown("Not legible"),
        known(Declared::date(day(), None).unwrap()),
    ] {
        for (old_end, new_end) in [(None, Some(end.clone())), (Some(end), None)] {
            let mut source = input();
            source.validity =
                MeasureValidity::new(unknown("Start omitted"), note("Duration"), old_end).unwrap();
            let original = MeasureValues::new(source);
            let before = original.clone();
            let changes = MeasureCorrectionValues::new(
                note("Also changed conditions"),
                MeasureValidity::new(unknown("Start omitted"), note("Duration"), new_end).unwrap(),
                note("Also changed supervision"),
            );
            assert_reason(original.correct_record(&changes), "correction_end_presence");
            assert_eq!(original, before);
        }
    }
}

#[test]
fn start_precision_can_increase_or_decrease_without_inventing_an_end() {
    for start in declarations() {
        let mut source = input();
        source.validity = MeasureValidity::new(start.clone(), note("V"), None).unwrap();
        let original = MeasureValues::new(source);
        for next in declarations() {
            if next == start {
                continue;
            }
            let validity = MeasureValidity::new(next.clone(), note("V"), None).unwrap();
            let corrected = original
                .correct_record(&replace_validity(&original, validity))
                .unwrap();
            assert_eq!(corrected.validity().start(), &next);
            assert_eq!(corrected.validity().end(), None);
        }
    }
}

#[test]
fn an_existing_end_can_change_precision_or_be_explicitly_unknown() {
    for end in declarations() {
        let mut source = input();
        source.validity =
            MeasureValidity::new(unknown("Start omitted"), note("V"), Some(end.clone())).unwrap();
        let original = MeasureValues::new(source);
        for next in declarations() {
            if next == end {
                continue;
            }
            let validity =
                MeasureValidity::new(unknown("Start omitted"), note("V"), Some(next.clone()))
                    .unwrap();
            let corrected = original
                .correct_record(&replace_validity(&original, validity))
                .unwrap();
            assert_eq!(corrected.validity().end(), Some(&next));
            assert_eq!(corrected.validity().start(), original.validity().start());
        }
    }
}

#[test]
fn changing_an_unknown_time_reason_is_a_real_permitted_change() {
    let mut source = input();
    source.validity = MeasureValidity::new(unknown("Not stated"), note("V"), None).unwrap();
    let original = MeasureValues::new(source);
    let validity = MeasureValidity::new(unknown("Illegible"), note("V"), None).unwrap();
    let corrected = original
        .correct_record(&replace_validity(&original, validity))
        .unwrap();
    assert_eq!(corrected.validity().start().declared(), Declared::unknown());
    assert_eq!(
        corrected.validity().start().unknown_reason(),
        Some(&note("Illegible"))
    );
}

#[test]
fn optional_offsets_and_explicit_clock_components_can_be_corrected() {
    let zones = [
        None,
        Some(UtcOffset::UTC),
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    ];
    for zone in zones {
        let mut source = input();
        source.validity = MeasureValidity::new(
            known(Declared::second(day(), 1, 2, 3, zone).unwrap()),
            note("V"),
            None,
        )
        .unwrap();
        let original = MeasureValues::new(source);
        for next_zone in zones {
            if next_zone == zone {
                continue;
            }
            let declared = Declared::second(day(), 1, 2, 3, next_zone).unwrap();
            let validity = MeasureValidity::new(known(declared), note("V"), None).unwrap();
            let corrected = original
                .correct_record(&replace_validity(&original, validity))
                .unwrap();
            assert_eq!(corrected.validity().start().declared(), declared);
        }
        let declared = Declared::second("2025-01-02".parse().unwrap(), 4, 5, 6, zone).unwrap();
        let validity = MeasureValidity::new(known(declared), note("V"), None).unwrap();
        let corrected = original
            .correct_record(&replace_validity(&original, validity))
            .unwrap();
        assert_eq!(corrected.validity().start().declared(), declared);
    }
}

#[test]
fn correction_preserves_each_catalog_kind_and_full_subject_reference() {
    let names = [
        "periodic_appearance",
        "financial_guarantee",
        "asset_seizure",
        "account_freeze",
        "travel_restriction",
        "custody_or_institution",
        "place_restriction",
        "contact_restriction",
        "home_separation",
        "public_office_suspension",
        "professional_suspension",
        "electronic_monitoring",
        "home_confinement",
        "pretrial_detention",
    ];
    for name in names {
        let mut source = input();
        source.kind = name.parse::<MeasureKind>().unwrap();
        let original = MeasureValues::new(source);
        let changes = MeasureCorrectionValues::new(
            note("Corrected"),
            original.validity().clone(),
            note("Corrected statement"),
        );
        let corrected = original.correct_record(&changes).unwrap();
        assert_eq!(corrected.kind(), original.kind());
        assert_eq!(corrected.subject(), original.subject());
    }
}
