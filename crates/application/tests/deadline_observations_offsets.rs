#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
use deadline_observation_support::*;

use application::{
    cases::CurrentCaseAdministration, deadline_inputs::DeadlineSourceDetail,
    hearing_results::hearing_result_receipt_matches,
    judicial_calendars::judicial_calendar_receipt_matches, procedural_facts::fact_receipt_matches,
};
use time::UtcOffset;

fn offset() -> UtcOffset {
    UtcOffset::from_hms(1, 0, 0).unwrap()
}

#[test]
fn the_same_fact_revision_cannot_change_its_recorded_offset_at_the_same_instant() {
    let (profile, mut material) = fixture();
    let head = fact_mut(&mut material.source_head);
    let metadata = inputs::metadata_mut(head);
    metadata.recorded_at = metadata.recorded_at.to_offset(offset());
    fact_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
    assert_eq!(material.source, material.source_head);
    assert!(build(&profile, &material, None).is_err());
}

#[test]
fn the_same_fact_revision_cannot_change_its_historical_administration_offset() {
    let (profile, mut material) = fixture();
    inputs::metadata_mut(fact_mut(&mut material.source)).recorded_administration =
        administration(inputs::case_id(), false);
    material.source_head = material.source.clone();
    let head = fact_mut(&mut material.source_head);
    let CurrentCaseAdministration::Recorded(admin) =
        &mut inputs::metadata_mut(head).recorded_administration
    else {
        unreachable!()
    };
    admin.changed_at = admin.changed_at.to_offset(offset());
    fact_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
    assert_eq!(material.source, material.source_head);
    assert!(build(&profile, &material, None).is_err());
}

#[test]
fn the_same_calendar_revision_cannot_change_its_recorded_offset_at_the_same_instant() {
    let (profile, mut material) = fixture();
    calendar_pair(&mut material);
    material.calendar_head = material.calendar.clone();
    let head = material.calendar_head.as_mut().unwrap();
    head.recorded_at = head.recorded_at.to_offset(offset());
    judicial_calendar_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
    assert_eq!(material.calendar, material.calendar_head);
    assert!(build(&profile, &material, None).is_err());
}

#[test]
fn the_same_result_revision_cannot_change_its_recorded_offset_at_the_same_instant() {
    let (profile, _) = fixture();
    let mut material = inputs::material(DeadlineSourceDetail::HearingResult(Box::new(
        hearing::fixture(),
    )));
    let Some(DeadlineSourceDetail::HearingResult(head)) = &mut material.source_head else {
        unreachable!()
    };
    head.snapshot.recorded_at = head.snapshot.recorded_at.to_offset(offset());
    hearing_result_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
    assert_eq!(material.source, material.source_head);
    assert!(build(&profile, &material, None).is_err());
}
