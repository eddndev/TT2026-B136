#![allow(dead_code)]
use crate::{
    deadline_observation_support as observed, deadline_support,
    deadline_technical_support::{self as technical, inputs},
};
use application::{
    deadline_currentness::{evaluate_deadline_currentness, DeadlineFreshness},
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::DeadlineProfileDefinition,
    deadline_technical::DeadlineReevaluationInputs,
    deadline_tracking::{TrackingDependency, TrackingPolicies, TrackingPolicy},
    deadlines::{deadline_capture_bytes, DeadlineChange, DeadlineDetail},
};
use time::OffsetDateTime;

pub fn checked_at() -> OffsetDateTime {
    crate::case_support::instant() + time::Duration::days(1)
}

pub fn assert_projection(
    base: &DeadlineDetail,
    resolved: Option<&DeadlineReevaluationInputs>,
    freshness: DeadlineFreshness,
    changed: &[TrackingDependency],
    due_at: Option<OffsetDateTime>,
) {
    let before = deadline_capture_bytes(inputs::hasher().as_ref(), base).unwrap();
    let result =
        evaluate_deadline_currentness(inputs::hasher().as_ref(), base, resolved, checked_at())
            .unwrap();
    assert_eq!(result.detail(), base);
    assert_eq!(
        deadline_capture_bytes(inputs::hasher().as_ref(), result.detail()).unwrap(),
        before
    );
    let projection = result.operational();
    assert_eq!(projection.freshness(), freshness);
    assert_eq!(projection.changed_dependencies(), changed);
    assert_eq!(projection.due_at(), due_at);
    assert_eq!(
        projection.checked_at(),
        if freshness == DeadlineFreshness::NotChecked {
            None
        } else {
            Some(checked_at())
        }
    );
}

pub fn profile_base(policy: TrackingPolicy) -> DeadlineDetail {
    let (command, preparation) = deadline_support::fixture();
    technical::human(
        command,
        preparation,
        Some(TrackingPolicies {
            profile: policy,
            ..technical::policies(TrackingPolicy::Follow)
        }),
        None,
    )
}

pub fn source_base(source: DeadlineSourceDetail, policy: TrackingPolicy) -> DeadlineDetail {
    let (mut command, mut preparation) = deadline_support::fixture();
    let request = inputs::request(&source);
    let resolved = preparation.resolved.as_mut().unwrap();
    let mut profile = observed::profile_input(&resolved.profile);
    profile.trigger = request.requirement;
    resolved.profile.definition = DeadlineProfileDefinition::new(profile).unwrap();
    observed::resign_profile(&mut resolved.profile);
    resolved.profile_head = resolved.profile.clone();
    resolved.material = inputs::material(source);
    let DeadlineChange::Register { definition } = &mut command.change else {
        panic!("registration expected")
    };
    definition.input.selection = request.trigger;
    technical::human(
        command,
        preparation,
        Some(technical::policies(policy)),
        None,
    )
}

pub fn unknown_source() -> DeadlineDetail {
    let (mut command, mut preparation) = deadline_support::fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        panic!("registration expected")
    };
    definition.input.selection.source = domain::procedural_facts::FactDeclaration::Unknown(
        technical::evaluation::text("Source not yet known"),
    );
    let material = &mut preparation.resolved.as_mut().unwrap().material;
    material.source = None;
    material.source_head = None;
    technical::human(
        command,
        preparation,
        Some(technical::policies(TrackingPolicy::Undetermined)),
        None,
    )
}
