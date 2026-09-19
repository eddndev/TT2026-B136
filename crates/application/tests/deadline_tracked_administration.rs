#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;
use application::{cases::*, deadlines::*};
use deadline_support::evaluation::inputs;
use deadline_tracked_support::{accepted, pending, resign};
use domain::{case_administration::CaseRevision, cases::CaseMetadata};

fn admin(revision: u32, closed: bool) -> CurrentCaseAdministration {
    let mut value = deadline_observation_support::administration(inputs::case_id(), closed);
    let CurrentCaseAdministration::Recorded(snapshot) = &mut value else {
        panic!()
    };
    snapshot.revision = CaseRevision::new(revision).unwrap();
    value
}
fn base() -> DeadlineDetail {
    let mut value = accepted();
    value.tracking.as_mut().unwrap().administration = admin(2, false);
    resign(&mut value);
    value
}
fn following(base: &DeadlineDetail, admin: CurrentCaseAdministration) -> DeadlineDetail {
    let mut value = pending(base);
    value.tracking.as_mut().unwrap().administration = admin;
    resign(&mut value);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
#[test]
fn a_technical_revision_can_observe_a_newer_closed_case_without_changing_history() {
    let base = base();
    let next = following(&base, admin(3, true));
    deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).unwrap();
    assert_eq!(next.calculation, base.calculation);
}
#[test]
fn a_recorded_administration_cannot_regress_or_return_to_an_unrevised_profile() {
    let base = base();
    for administration in [
        admin(1, false),
        base.calculation.material.administration.clone(),
    ] {
        let next = following(&base, administration);
        assert!(deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).is_err());
    }
}
#[test]
fn the_same_administrative_revision_cannot_change_its_captured_offset() {
    let base = base();
    let mut administration = admin(2, false);
    let CurrentCaseAdministration::Recorded(value) = &mut administration else {
        panic!()
    };
    value.changed_at = value
        .changed_at
        .to_offset(time::UtcOffset::from_hms(3, 0, 0).unwrap());
    assert_eq!(
        administration,
        base.tracking.as_ref().unwrap().administration
    );
    let next = following(&base, administration);
    assert!(deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).is_err());
}
#[test]
fn the_same_administrative_revision_cannot_replace_values_or_author_metadata() {
    let base = base();
    for change_values in [false, true] {
        let mut administration = admin(2, change_values);
        if !change_values {
            let CurrentCaseAdministration::Recorded(value) = &mut administration else {
                panic!()
            };
            value.changed_by.email = "changed@example.com".into();
        }
        let next = following(&base, administration);
        assert!(deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).is_err());
    }
}
#[test]
fn an_unrevised_administration_can_gain_a_revision_but_cannot_be_rewritten() {
    let base = accepted();
    let next = following(&base, admin(1, false));
    deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).unwrap();
    let changed =
        CurrentCaseAdministration::Unrevised(CaseMetadata::new("Changed", "REF-CHANGED").unwrap());
    let next = following(&base, changed);
    assert!(deadline_successor_matches(inputs::hasher().as_ref(), &base, &next).is_err());
}
