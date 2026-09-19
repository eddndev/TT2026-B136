#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_currentness_support;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_currentness::DeadlineFreshness::{Changed, Current, NotChecked},
    deadline_inputs::DeadlineSourceDetail,
    deadline_technical::{DeadlineReevaluationNoChange, DeadlineReevaluationOutcome},
    deadline_tracking::{DeadlineReviewState, TrackingDependency, TrackingPolicy},
};
use deadline_currentness_support::*;
use deadline_observation_support as observed;
use deadline_technical_support::*;

#[test]
fn exact_heads_expose_the_accepted_due_without_changing_the_capture() {
    let base = accepted(TrackingPolicy::Follow);
    assert!(base.calculation.result.due_at().is_some());
    assert_projection(
        &base,
        Some(&heads(&base)),
        Current,
        &[],
        base.calculation.result.due_at(),
    );
}

#[test]
fn every_dependency_preserves_fixed_advances_but_blocks_follow_and_retirement() {
    for policy in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        for withdrawn in [false, true] {
            let profile = profile_base(policy);
            let mut profile_heads = heads(&profile);
            observed::replace_profile(&mut profile_heads.profile_head, withdrawn);
            let source = accepted(policy);
            let source_inputs = source_heads(&source, 2, withdrawn);
            let calendar = calendar_base(TrackingPolicy::Follow, policy);
            let calendar_inputs = calendar_heads(&calendar, 2, withdrawn);
            let notification = notification_base(policy);
            let mut parent_inputs = heads(&notification);
            parent_inputs.notification_parent_head =
                Some(inputs::resolution(2, withdrawn, "2026-01-01"));
            for (base, resolved, dependency) in [
                (profile, profile_heads, TrackingDependency::Profile),
                (source, source_inputs, TrackingDependency::Source),
                (calendar, calendar_inputs, TrackingDependency::Calendar),
                (notification, parent_inputs, TrackingDependency::Source),
            ] {
                let blocks = withdrawn || policy == TrackingPolicy::Follow;
                let changed = [dependency];
                assert_projection(
                    &base,
                    Some(&resolved),
                    if blocks { Changed } else { Current },
                    if blocks { &changed } else { &[] },
                    if blocks {
                        None
                    } else {
                        base.calculation.result.due_at()
                    },
                );
            }
        }
    }
}

#[test]
fn hearing_result_heads_follow_the_same_source_policy() {
    for policy in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        for withdrawn in [false, true] {
            let base = source_base(
                DeadlineSourceDetail::HearingResult(Box::new(inputs::hearing(
                    1,
                    false,
                    "2026-01-06",
                    &[],
                ))),
                policy,
            );
            let mut resolved = heads(&base);
            resolved.material.source_head = Some(DeadlineSourceDetail::HearingResult(Box::new(
                inputs::hearing(2, withdrawn, "2026-01-07", &[]),
            )));
            let blocks = withdrawn || policy == TrackingPolicy::Follow;
            assert_projection(
                &base,
                Some(&resolved),
                if blocks { Changed } else { Current },
                if blocks {
                    &[TrackingDependency::Source]
                } else {
                    &[]
                },
                if blocks {
                    None
                } else {
                    base.calculation.result.due_at()
                },
            );
        }
    }
}

#[test]
fn notification_heads_cannot_hide_withdrawal_behind_fixed_source_policy() {
    for policy in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        for withdrawn in [false, true] {
            let base = notification_base(policy);
            let mut resolved = heads(&base);
            resolved.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                inputs::notification(2, withdrawn, 1, "2026-01-07"),
            )));
            resolved.notification_parent_head = Some(inputs::resolution(1, false, "2026-01-01"));
            let blocks = withdrawn || policy == TrackingPolicy::Follow;
            assert_projection(
                &base,
                Some(&resolved),
                if blocks { Changed } else { Current },
                if blocks {
                    &[TrackingDependency::Source]
                } else {
                    &[]
                },
                if blocks {
                    None
                } else {
                    base.calculation.result.due_at()
                },
            );
        }
    }
}

#[test]
fn notification_and_parent_advances_are_one_source_change() {
    let base = notification_base(TrackingPolicy::Follow);
    let mut resolved = heads(&base);
    resolved.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
        inputs::notification(2, false, 2, "2026-01-07"),
    )));
    resolved.notification_parent_head = Some(inputs::resolution(2, false, "2026-01-01"));
    assert_projection(
        &base,
        Some(&resolved),
        Changed,
        &[TrackingDependency::Source],
        None,
    );
}

#[test]
fn simultaneous_changes_are_reported_in_dependency_order() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let mut resolved = source_heads(&base, 2, false);
    observed::replace_profile(&mut resolved.profile_head, false);
    resolved.material.calendar_head = calendar_heads(&base, 2, false).material.calendar_head;
    assert_projection(
        &base,
        Some(&resolved),
        Changed,
        &[
            TrackingDependency::Profile,
            TrackingDependency::Source,
            TrackingDependency::Calendar,
        ],
        None,
    );
}

#[test]
fn a_matching_pending_revision_is_current_without_acceptance_or_due() {
    for withdrawn in [false, true] {
        let initial = accepted(TrackingPolicy::Follow);
        let resolved = source_heads(&initial, 2, withdrawn);
        let pending = revision(
            &initial,
            event_command(source_event(&resolved, 2)),
            resolved.clone(),
        );
        assert_eq!(pending.review_state(), DeadlineReviewState::Pending);
        assert_projection(&pending, Some(&resolved), Current, &[], None);
    }
}

#[test]
fn a_calendar_recalculation_becomes_current_without_inventing_a_blocked_due() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let resolved = calendar_heads(&base, 2, false);
    assert_projection(
        &base,
        Some(&resolved),
        Changed,
        &[TrackingDependency::Calendar],
        None,
    );
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 2);
    let recalculated = revision(&base, event_command(event), resolved);
    assert_eq!(recalculated.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(recalculated.calculation.result.due_at(), None);
    assert_projection(
        &recalculated,
        Some(&heads(&recalculated)),
        Current,
        &[],
        None,
    );
}

#[test]
fn absent_source_and_calendar_do_not_invent_dependencies_or_a_due() {
    let base = unknown_source();
    assert_eq!(base.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(
        base.tracking.as_ref().unwrap().observations.entries.len(),
        1
    );
    assert_projection(&base, Some(&heads(&base)), Current, &[], None);
}

#[test]
fn legacy_and_retired_records_are_verified_without_current_inputs() {
    for base in [
        legacy(),
        attention(&legacy()),
        retired(&accepted(TrackingPolicy::Follow)),
    ] {
        assert_projection(&base, None, NotChecked, &[], None);
    }
}

#[test]
fn a_bootstrapped_legacy_record_can_be_current_but_never_accepted() {
    let base = legacy();
    let pending = revision(&base, bootstrap(), heads(&base));
    assert_eq!(pending.review_state(), DeadlineReviewState::Pending);
    assert_projection(&pending, Some(&heads(&pending)), Current, &[], None);
    assert_projection(
        &pending,
        Some(&source_heads(&pending, 2, false)),
        Changed,
        &[TrackingDependency::Source],
        None,
    );
}

#[test]
fn an_old_event_no_change_does_not_certify_newer_checked_heads() {
    let initial = accepted(TrackingPolicy::Follow);
    let second = source_heads(&initial, 2, false);
    let old_event = event_command(source_event(&second, 2));
    let base = revision(&initial, old_event.clone(), second);
    let resolved = source_heads(&base, 3, false);
    assert!(matches!(
        prepare(&base, old_event, resolved.clone()).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::AlreadyObserved)
    ));
    assert_projection(
        &base,
        Some(&resolved),
        Changed,
        &[TrackingDependency::Source],
        None,
    );
}

#[test]
fn an_unselected_event_no_change_does_not_hide_a_selected_source_advance() {
    let base = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&base, 2, false);
    let mut event = source_event(&resolved, 2);
    event.source_id = uuid::Uuid::from_u128(999);
    assert!(matches!(
        prepare(&base, event_command(event), resolved.clone()).unwrap(),
        DeadlineReevaluationOutcome::NoChange(DeadlineReevaluationNoChange::DependencyNotSelected)
    ));
    assert_projection(
        &base,
        Some(&resolved),
        Changed,
        &[TrackingDependency::Source],
        None,
    );
}

#[test]
fn a_new_closed_administration_does_not_retire_an_active_deadline() {
    let base = accepted(TrackingPolicy::Follow);
    let mut resolved = heads(&base);
    resolved.material.administration = observed::administration(base.case_id, true);
    assert_projection(
        &base,
        Some(&resolved),
        Current,
        &[],
        base.calculation.result.due_at(),
    );
}
