#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    cases::CurrentCaseAdministration, deadline_tracking::TrackingPolicy, deadlines::DeadlineDetail,
};
use deadline_observation_support as observed;
use deadline_technical_support::*;
use domain::case_administration::CaseRevision;

fn base() -> DeadlineDetail {
    let (command, mut preparation) = deadline_support::fixture();
    preparation.administration = observed::administration(inputs::case_id(), false);
    let CurrentCaseAdministration::Recorded(admin) = &mut preparation.administration else {
        unreachable!();
    };
    admin.revision = CaseRevision::new(2).unwrap();
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = preparation.administration.clone();
    human(
        command,
        preparation,
        Some(policies(TrackingPolicy::Follow)),
        None,
    )
}
fn inconsistent_inputs(
    base: &DeadlineDetail,
    offset_only: bool,
) -> application::deadline_technical::DeadlineReevaluationInputs {
    let mut resolved = heads(base);
    let CurrentCaseAdministration::Recorded(admin) = &mut resolved.material.administration else {
        unreachable!();
    };
    if offset_only {
        admin.changed_at = admin
            .changed_at
            .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    } else {
        admin.revision = CaseRevision::FIRST;
    }
    resolved
}

#[test]
fn an_already_observed_event_cannot_hide_inconsistent_administration_inputs() {
    let base = base();
    for offset_only in [false, true] {
        let resolved = inconsistent_inputs(&base, offset_only);
        let event = source_event(&resolved, 1);
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "already-observed outcome hid inconsistent administration: offset_only={offset_only}"
        );
    }
}

#[test]
fn an_unselected_event_cannot_hide_inconsistent_administration_inputs() {
    let base = base();
    for offset_only in [false, true] {
        let resolved = inconsistent_inputs(&base, offset_only);
        let mut event = source_event(&resolved, 1);
        event.source_id = uuid::Uuid::from_u128(999);
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "unselected outcome hid inconsistent administration: offset_only={offset_only}"
        );
    }
}
