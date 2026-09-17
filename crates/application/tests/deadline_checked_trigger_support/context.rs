use super::{inconsistent, recorded, selection, unknown};
use crate::deadline_input_support::{calendar, case_id, hasher, material, resolution};
use application::{
    cases::CurrentCaseAdministration,
    deadline_inputs::{extract_checked_deadline_inputs, DeadlineCalendarRef, DeadlineSourceDetail},
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    crypto::Sha256Digest,
    deadline_triggers::{TriggerBlock, TriggerField, TriggerOutcome, TriggerRequirement},
    judicial_calendars::JudicialCalendarClassification,
};
use uuid::Uuid;

#[test]
fn an_unknown_trigger_still_exposes_only_the_exact_selected_calendar() {
    let (selection, mut material) = unknown();
    let exact = calendar(1, false, JudicialCalendarClassification::Countable);
    let selected = DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    };
    material.calendar = Some(exact);
    material.calendar_head = Some(calendar(3, true, JudicialCalendarClassification::Excluded));
    let before = material.clone();
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    let checked = extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        Some(selected),
        &material,
    )
    .unwrap();
    assert_eq!(
        checked.extraction().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    assert!(checked.extraction().source().is_none());
    let expected = &material.calendar.as_ref().unwrap().values;
    assert_eq!(checked.calendar(), Some(expected));
    assert!(std::ptr::eq(checked.calendar().unwrap(), expected));
    assert_eq!(material, before);
}

#[test]
fn unknown_source_does_not_hide_a_corrupt_exact_or_head_calendar_receipt() {
    for corrupt_head in [false, true] {
        let (selection, mut material) = unknown();
        let exact = calendar(1, false, JudicialCalendarClassification::Countable);
        let selected = DeadlineCalendarRef {
            id: exact.id,
            revision: exact.revision,
        };
        material.calendar = Some(exact);
        material.calendar_head = Some(calendar(2, false, JudicialCalendarClassification::Excluded));
        let item = if corrupt_head {
            &mut material.calendar_head
        } else {
            &mut material.calendar
        };
        item.as_mut().unwrap().receipt.submission_digest = Sha256Digest::from_array([255; 32]);
        let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
        inconsistent(extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            Some(selected),
            &material,
        ));
    }
}

#[test]
fn calendar_selection_and_presence_are_checked_even_without_a_known_source() {
    for kind in 0..4 {
        let (selection, mut material) = unknown();
        let exact = calendar(1, false, JudicialCalendarClassification::Countable);
        let mut selected = Some(DeadlineCalendarRef {
            id: exact.id,
            revision: exact.revision,
        });
        material.calendar = Some(exact.clone());
        material.calendar_head = Some(exact);
        match kind {
            0 => selected = None,
            1 => material.calendar = None,
            2 => material.calendar_head = None,
            3 => {
                selected.as_mut().unwrap().id =
                    domain::judicial_calendars::JudicialCalendarId::from_uuid(Uuid::nil())
            }
            _ => unreachable!(),
        }
        let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
        inconsistent(extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            selected,
            &material,
        ));
    }
}

#[test]
fn closed_administration_is_valid_for_reading_but_cannot_be_corrupt_or_cross_case() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let (requirement, selection) = selection(&source);
    let mut material = material(source);
    material.administration = recorded(case_id(), CaseAdministrativeStatus::Closed);
    assert!(extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        None,
        &material
    )
    .is_ok());
    for wrong_case in [false, true] {
        let (selection, mut material) = unknown();
        let case = if wrong_case {
            CaseId::from_uuid(Uuid::from_u128(99))
        } else {
            case_id()
        };
        material.administration = recorded(case, CaseAdministrativeStatus::Closed);
        if !wrong_case {
            let CurrentCaseAdministration::Recorded(snapshot) = &mut material.administration else {
                unreachable!()
            };
            snapshot.values_digest = Sha256Digest::from_array([255; 32]);
        }
        inconsistent(extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            None,
            &material,
        ));
    }
}

#[test]
fn mismatched_material_case_is_rejected_even_with_no_source_or_calendar() {
    let (selection, mut material) = unknown();
    material.case_id = CaseId::from_uuid(Uuid::from_u128(99));
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    inconsistent(extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        None,
        &material,
    ));
}
