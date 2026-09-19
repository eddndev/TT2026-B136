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
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent, DeadlineFreshness},
    deadline_tracking::TrackingPolicy,
};
use deadline_currentness_support::*;
use deadline_observation_support as observed;
use deadline_technical_support::*;

#[test]
fn current_evidence_distinguishes_required_review_from_calendar_recalculation() {
    for policy in [TrackingPolicy::Fixed, TrackingPolicy::Follow] {
        for withdrawn in [false, true] {
            let base = calendar_base(TrackingPolicy::Follow, policy);
            let resolved = calendar_heads(&base, 2, withdrawn);
            let result = evaluate_deadline_currentness(
                inputs::hasher().as_ref(),
                &base,
                Some(&resolved),
                checked_at(),
            )
            .unwrap();
            assert_eq!(result.operational().requires_review(), withdrawn);
            assert_eq!(
                result.operational().freshness(),
                if withdrawn || policy == TrackingPolicy::Follow {
                    DeadlineFreshness::Changed
                } else {
                    DeadlineFreshness::Current
                }
            );
            assert_eq!(result.detail(), &base);
        }
    }
}

#[test]
fn source_profile_parent_and_captured_pending_require_human_review() {
    let source = accepted(TrackingPolicy::Follow);
    let source_inputs = source_heads(&source, 2, false);
    let profile = profile_base(TrackingPolicy::Follow);
    let mut profile_inputs = heads(&profile);
    observed::replace_profile(&mut profile_inputs.profile_head, false);
    let parent = notification_base(TrackingPolicy::Follow);
    let mut parent_inputs = heads(&parent);
    parent_inputs.notification_parent_head = Some(inputs::resolution(2, false, "2026-01-01"));
    let pending = revision(
        &source,
        event_command(source_event(&source_inputs, 2)),
        source_inputs.clone(),
    );
    for (base, resolved) in [
        (source, source_inputs.clone()),
        (profile, profile_inputs),
        (parent, parent_inputs),
        (pending, source_inputs),
    ] {
        let result = evaluate_deadline_currentness(
            inputs::hasher().as_ref(),
            &base,
            Some(&resolved),
            checked_at(),
        )
        .unwrap();
        assert!(result.operational().requires_review());
        assert_eq!(result.operational().due_at(), None);
        assert_eq!(result.detail(), &base);
    }
}

#[test]
fn fixed_active_legacy_retired_and_historical_reads_do_not_invent_review() {
    let fixed = accepted(TrackingPolicy::Fixed);
    let resolved = source_heads(&fixed, 2, false);
    let result = evaluate_deadline_currentness(
        inputs::hasher().as_ref(),
        &fixed,
        Some(&resolved),
        checked_at(),
    )
    .unwrap();
    assert!(!result.operational().requires_review());
    for base in [legacy(), retired(&fixed)] {
        let result =
            evaluate_deadline_currentness(inputs::hasher().as_ref(), &base, None, checked_at())
                .unwrap();
        assert!(!result.operational().requires_review());
    }
    let initial = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&initial, 2, false);
    let pending = revision(
        &initial,
        event_command(source_event(&resolved, 2)),
        resolved,
    );
    assert!(
        !DeadlineCurrent::historical(inputs::hasher().as_ref(), &pending)
            .unwrap()
            .operational()
            .requires_review()
    );
}
