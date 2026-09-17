mod deadline_trigger_integrity_support;
mod hearing_result_support;

use deadline_trigger_integrity_support::*;
use domain::{
    deadline_triggers::*,
    hearing_results::*,
    hearings::HearingId,
    judicial_calendars::CivilDate,
    procedural_facts::{FactDeclaration, FactHearingRef},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use hearing_result_support::{agreement, input};
use time::{macros::date, UtcOffset};
use uuid::Uuid;

fn values() -> HearingResultValues {
    let mut value = input();
    value.agreements = vec![
        agreement(0, "Exact zero agreement"),
        agreement(91, "Second agreement"),
    ];
    HearingResultValues::new(value).unwrap()
}
fn selected(agreement_id: Option<u128>) -> TriggerSelection {
    let mut reference = hearing();
    reference.agreement_id =
        agreement_id.map(|id| HearingResultAgreementId::from_uuid(Uuid::from_u128(id)));
    selection(TriggerSourceRef::HearingResult(reference))
}
fn extract(values: &HearingResultValues, selection: &TriggerSelection) -> TriggerExtraction {
    extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime),
        selection,
        Some(hearing_material(values)),
    )
    .unwrap()
}

#[test]
fn hearing_material_requires_exact_case_hearing_result_and_revision() {
    let values = values();
    let reference = hearing();
    for (case_id, hearing_id, result_id, revision, expected) in [
        (
            case(2),
            reference.hearing_id,
            reference.result_id,
            reference.revision,
            TriggerIntegrityError::CaseMismatch,
        ),
        (
            case(1),
            HearingId::from_uuid(Uuid::from_u128(301)),
            reference.result_id,
            reference.revision,
            TriggerIntegrityError::SourceMismatch,
        ),
        (
            case(1),
            reference.hearing_id,
            HearingResultId::from_uuid(Uuid::from_u128(401)),
            reference.revision,
            TriggerIntegrityError::SourceMismatch,
        ),
        (
            case(1),
            reference.hearing_id,
            reference.result_id,
            HearingResultRevision::new(6).unwrap(),
            TriggerIntegrityError::SourceMismatch,
        ),
    ] {
        let material = TriggerMaterial::HearingResult {
            case_id,
            hearing_id,
            result_id,
            revision,
            values: &values,
            digests: hearing_digests(),
        };
        reject(
            TriggerField::ResolutionIssuedAt,
            &selected(None),
            Some(material),
            expected,
        );
    }
}

#[test]
fn missing_agreement_is_integrity_failure_before_incompatible_requirement() {
    let values = values();
    reject(
        TriggerField::ResolutionIssuedAt,
        &selected(Some(42)),
        Some(hearing_material(&values)),
        TriggerIntegrityError::MissingAgreement,
    );
    reject(
        TriggerField::HearingSessionEventTime,
        &selected(Some(42)),
        Some(hearing_material(&values)),
        TriggerIntegrityError::MissingAgreement,
    );
}

#[test]
fn no_agreement_and_a_selected_zero_uuid_remain_distinct() {
    let values = values();
    for (id, expected) in [
        (None, None),
        (Some(0), Some(agreement(0, "Exact zero agreement"))),
        (Some(91), Some(agreement(91, "Second agreement"))),
    ] {
        let selected = selected(id);
        let result = extract(&values, &selected);
        let source = result.source().unwrap();
        assert_eq!(source.case_id, case(1));
        let FactDeclaration::Known(reference) = selected.source else {
            panic!("known source")
        };
        assert_eq!(source.reference, reference);
        assert_eq!(source.agreement, expected);
        assert_eq!(
            source.digests,
            TriggerDigests::HearingResult(hearing_digests())
        );
        assert_eq!(
            source.provenance,
            TriggerProvenance::HearingResult(values.provenance().clone())
        );
        assert_eq!(source.stated_effect, None);
    }
}

#[test]
fn zero_agreement_must_exist_and_is_not_a_no_selection_sentinel() {
    let empty = HearingResultValues::new(input()).unwrap();
    reject(
        TriggerField::HearingSessionEventTime,
        &selected(Some(0)),
        Some(hearing_material(&empty)),
        TriggerIntegrityError::MissingAgreement,
    );
    assert_eq!(
        extract(&empty, &selected(None)).source().unwrap().agreement,
        None
    );
}

#[test]
fn same_agreement_id_cannot_substitute_a_different_result_revision() {
    let old_values = values();
    let old = extract(&old_values, &selected(Some(0)));
    let mut revised = input();
    revised.agreements = vec![agreement(0, "Corrected exact text")];
    let revised = HearingResultValues::new(revised).unwrap();
    let reference = hearing();
    let revision = HearingResultRevision::new(6).unwrap();
    let material = TriggerMaterial::HearingResult {
        case_id: case(1),
        hearing_id: reference.hearing_id,
        result_id: reference.result_id,
        revision,
        values: &revised,
        digests: hearing_digests(),
    };
    reject(
        TriggerField::HearingSessionEventTime,
        &selected(Some(0)),
        Some(material),
        TriggerIntegrityError::SourceMismatch,
    );
    let reference = FactHearingRef {
        revision,
        agreement_id: Some(HearingResultAgreementId::from_uuid(Uuid::nil())),
        ..reference
    };
    let latest = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime),
        &selection(TriggerSourceRef::HearingResult(reference)),
        Some(material),
    )
    .unwrap();
    assert_eq!(
        latest.source().unwrap().agreement,
        Some(agreement(0, "Corrected exact text"))
    );
    assert_eq!(
        old.source().unwrap().agreement,
        Some(agreement(0, "Exact zero agreement"))
    );
}

#[test]
fn hearing_dates_preserve_the_local_day_and_offset_without_using_bounds() {
    for seconds in [-50400, -21600, 0, 19800, 50400] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        let mut value = input();
        value.event_time = DeclaredHearingResultTime::date(date!(2028 - 02 - 29), offset).unwrap();
        let result = extract(&HearingResultValues::new(value).unwrap(), &selected(None));
        let expected = DeclaredProceduralTime::date(
            CivilDate::from_date(date!(2028 - 02 - 29)).unwrap(),
            Some(offset),
        )
        .unwrap();
        assert_eq!(
            result.outcome(),
            &TriggerOutcome::Extracted { at: expected }
        );
        let TriggerOutcome::Extracted { at } = result.outcome() else {
            panic!("extracted time")
        };
        assert_eq!(at.precision(), DeclaredProceduralPrecision::Date);
        assert_eq!(at.local_hour(), None);
        assert_eq!(at.local_minute(), None);
        assert_eq!(at.local_second(), None);
        assert_eq!(at.instant_value(), None);
        assert_eq!(at.offset(), Some(offset));
    }
}

#[test]
fn hearing_instants_preserve_original_components_seconds_and_offset() {
    for seconds in [-50400, -20700, 0, 19800, 50400] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        for (hour, minute, second) in [(0, 0, 0), (23, 59, 37)] {
            let day = date!(2026 - 01 - 01);
            let instant = day
                .with_hms(hour, minute, second)
                .unwrap()
                .assume_offset(offset);
            let mut value = input();
            value.event_time = DeclaredHearingResultTime::instant(instant).unwrap();
            let result = extract(&HearingResultValues::new(value).unwrap(), &selected(None));
            let expected = DeclaredProceduralTime::second(
                CivilDate::from_date(day).unwrap(),
                hour,
                minute,
                second,
                Some(offset),
            )
            .unwrap();
            assert_eq!(
                result.outcome(),
                &TriggerOutcome::Extracted { at: expected }
            );
            let TriggerOutcome::Extracted { at } = result.outcome() else {
                panic!("extracted time")
            };
            assert_eq!(at.precision(), DeclaredProceduralPrecision::Second);
            assert_eq!(at.offset(), Some(offset));
            assert_eq!(at.instant_value(), Some(instant));
        }
    }
}

#[test]
fn hearing_date_conversion_preserves_both_supported_year_boundaries() {
    for day in [date!(0001 - 01 - 01), date!(9999 - 12 - 31)] {
        let mut value = input();
        value.event_time = DeclaredHearingResultTime::date(day, UtcOffset::UTC).unwrap();
        let result = extract(&HearingResultValues::new(value).unwrap(), &selected(None));
        let expected =
            DeclaredProceduralTime::date(CivilDate::from_date(day).unwrap(), Some(UtcOffset::UTC))
                .unwrap();
        assert_eq!(
            result.outcome(),
            &TriggerOutcome::Extracted { at: expected }
        );
    }
}

#[test]
fn session_event_time_does_not_depend_on_inferred_occurrence_or_completion() {
    for (occurrence, extent) in [
        (
            HearingResultOccurrence::NotStarted,
            HearingResultExtent::Unspecified,
        ),
        (
            HearingResultOccurrence::Occurred,
            HearingResultExtent::Partial,
        ),
        (
            HearingResultOccurrence::Occurred,
            HearingResultExtent::Concluded,
        ),
    ] {
        let mut value = input();
        value.occurrence = occurrence;
        value.extent = extent;
        value.agreements = vec![agreement(0, "Hearing ended on 2030-01-01 at 12:00:00")];
        let expected = DeclaredProceduralTime::date(
            CivilDate::from_date(value.event_time.local_date()).unwrap(),
            Some(value.event_time.offset()),
        )
        .unwrap();
        let result = extract(
            &HearingResultValues::new(value).unwrap(),
            &selected(Some(0)),
        );
        assert_eq!(
            result.outcome(),
            &TriggerOutcome::Extracted { at: expected }
        );
    }
}

#[test]
fn concluded_session_does_not_supply_an_undeclared_hearing_end() {
    let mut value = input();
    value.extent = HearingResultExtent::Concluded;
    let value = HearingResultValues::new(value).unwrap();
    let result = extract_trigger_time(
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::HearingResult,
        },
        &selected(None),
        Some(hearing_material(&value)),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::MissingQualification {
            purpose: QualifiedTriggerPurpose::HearingEnd
        })
    );
    assert!(result.source().is_some());
}
