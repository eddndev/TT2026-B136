#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
mod deadline_tracked_support;

use application::{
    deadline_inputs::DeadlineSourceDetail, deadline_reevaluation::*, deadline_tracking::*,
    deadlines::*,
};
use deadline_support::evaluation::{inputs, label, text};
use deadline_tracked_support::{accepted, capture, legacy, pending, resign};
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

fn manual_successor(base: &DeadlineDetail, action: DeadlineAction) -> DeadlineDetail {
    let mut value = base.clone();
    value.revision = base.revision.next().unwrap();
    value.reason = Some(text("Explicit human followup"));
    value.receipt.operation_id =
        DeadlineOperationId::from_uuid(Uuid::from_u128(200 + u128::from(value.revision.get())));
    value.receipt.action = action;
    value.receipt.expected_revision = base.revision.get();
    value.recorded_by = DeadlineActorSnapshot::User {
        id: inputs::actor(),
        email: "owner@example.com".into(),
    };
    value.status = if action == DeadlineAction::Retire {
        DeadlineStatus::Retired
    } else {
        DeadlineStatus::Active
    };
    if action == DeadlineAction::SetAttention {
        value.attention = deadline_support::attention();
    }
    if value.tracking.is_none() {
        value.tracking = Some(capture(base));
    }
    value.receipt.version = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: Sha256Digest::from_array([0; 32]),
        predecessor: Some(PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        }),
        cause: None,
    });
    resign(&mut value);
    value
}

fn assert_record_valid(value: &DeadlineDetail) {
    deadline_receipt_matches(inputs::hasher().as_ref(), value).unwrap();
}

fn assert_successor(previous: &DeadlineDetail, next: &DeadlineDetail) {
    assert_record_valid(previous);
    assert_record_valid(next);
    deadline_successor_matches(inputs::hasher().as_ref(), previous, next).unwrap();
}

fn assert_rejected_link(previous: &DeadlineDetail, next: &DeadlineDetail) {
    assert_record_valid(previous);
    assert_record_valid(next);
    assert!(deadline_successor_matches(inputs::hasher().as_ref(), previous, next).is_err());
}

#[test]
fn a_tracked_reevaluation_can_follow_a_verified_legacy_revision() {
    let previous = legacy();
    let mut next = pending(&previous);
    let mut material = previous.calculation.material.clone();
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        false,
        "2026-01-10",
    ))));
    let tracking = next.tracking.as_mut().unwrap();
    tracking.observations = application::deadline_observations::build_deadline_observations(
        inputs::hasher().as_ref(),
        previous.case_id,
        &previous.calculation.profile,
        &material,
        None,
    )
    .unwrap();
    tracking.policies = TrackingPolicies {
        profile: TrackingPolicy::Undetermined,
        source: TrackingPolicy::Undetermined,
        calendar: TrackingPolicy::Undetermined,
    };
    tracking.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        [TrackingDependency::Profile, TrackingDependency::Source]
            .map(|dependency| TrackingReviewRequirement {
                dependency,
                reason: TrackingReviewReason::PolicyUndetermined,
            })
            .to_vec(),
    )
    .unwrap();
    resign(&mut next);
    assert_successor(&previous, &next);
    assert_eq!(next.calculation, previous.calculation);
}

#[test]
fn tracked_revisions_share_one_chain_across_technical_and_human_actions() {
    let first = accepted();
    let second = pending(&first);
    let third = manual_successor(&second, DeadlineAction::SetAttention);
    let fourth = manual_successor(&third, DeadlineAction::Retire);
    assert_successor(&first, &second);
    assert_successor(&second, &third);
    assert_successor(&third, &fourth);
    assert_eq!(second.revision.get(), 2);
    assert_eq!(third.revision.get(), 3);
    assert_eq!(fourth.revision.get(), 4);
}

#[test]
fn both_predecessor_digests_are_checked_even_after_resigning_the_successor() {
    for previous in [legacy(), accepted()] {
        for change_capture in [false, true] {
            let mut next = pending(&previous);
            let DeadlineReceiptVersion::Tracked(metadata) = &mut next.receipt.version else {
                panic!("tracked successor expected");
            };
            let predecessor = metadata.predecessor.as_mut().unwrap();
            if change_capture {
                predecessor.capture_digest = Sha256Digest::from_array([0x55; 32]);
            } else {
                predecessor.submission_digest = Sha256Digest::from_array([0x66; 32]);
            }
            resign(&mut next);
            assert_rejected_link(&previous, &next);
        }
    }
}

#[test]
fn another_root_or_a_reused_operation_cannot_link_valid_individual_receipts() {
    let previous = accepted();
    for change_root in [false, true] {
        let mut next = pending(&previous);
        if change_root {
            next.id = DeadlineId::from_uuid(Uuid::from_u128(999));
        } else {
            next.receipt.operation_id = previous.receipt.operation_id;
        }
        resign(&mut next);
        assert_rejected_link(&previous, &next);
    }
}

#[test]
fn a_skipped_revision_cannot_link_even_when_its_receipt_counter_is_consistent() {
    let previous = accepted();
    let mut next = pending(&previous);
    next.revision = DeadlineRevision::new(3).unwrap();
    next.receipt.expected_revision = 2;
    resign(&mut next);
    assert_rejected_link(&previous, &next);
}

#[test]
fn a_cross_case_record_is_rejected_before_it_can_supply_a_chain_link() {
    let previous = accepted();
    let mut next = pending(&previous);
    next.case_id = CaseId::from_uuid(Uuid::from_u128(900));
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &next).is_err());
    assert!(deadline_successor_matches(inputs::hasher().as_ref(), &previous, &next).is_err());
}

#[test]
fn a_retired_predecessor_cannot_be_reopened_by_a_valid_technical_record() {
    let first = accepted();
    let previous = manual_successor(&first, DeadlineAction::Retire);
    assert_successor(&first, &previous);
    let mut next = pending(&previous);
    next.status = DeadlineStatus::Active;
    resign(&mut next);
    assert_rejected_link(&previous, &next);
}

#[test]
fn pending_reevaluation_preserves_attention_responsible_definition_and_calculation() {
    let previous = manual_successor(&accepted(), DeadlineAction::SetAttention);
    let original = pending(&previous);
    assert_successor(&previous, &original);
    assert_eq!(original.attention, previous.attention);
    assert_eq!(original.responsible, previous.responsible);
    assert_eq!(original.definition, previous.definition);
    assert_eq!(original.calculation, previous.calculation);
    for mutation in 0..4 {
        let mut next = original.clone();
        match mutation {
            0 => next.attention = DeadlineAttention::Pending,
            1 => next.responsible.email = "another-snapshot@example.com".into(),
            2 => next.definition.title = label("A silently replaced title"),
            _ => {
                next.calculation.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                    inputs::resolution(2, false, "2026-01-10"),
                )));
            }
        }
        resign(&mut next);
        assert_rejected_link(&previous, &next);
    }
}

#[test]
fn attention_and_retirement_keep_pending_review_policies_and_observations() {
    let previous = pending(&accepted());
    for action in [DeadlineAction::SetAttention, DeadlineAction::Retire] {
        let next = manual_successor(&previous, action);
        assert_successor(&previous, &next);
        assert_eq!(next.tracking, previous.tracking);
        assert_eq!(next.calculation, previous.calculation);
        let DeadlineReceiptVersion::Tracked(metadata) = &next.receipt.version else {
            panic!("tracked manual successor expected");
        };
        assert!(metadata.cause.is_none());
        assert!(matches!(
            next.recorded_by,
            DeadlineActorSnapshot::User { .. }
        ));
    }
}

#[test]
fn manual_attention_or_retirement_cannot_replace_tracking_with_a_valid_new_capture() {
    let previous = pending(&accepted());
    for action in [DeadlineAction::SetAttention, DeadlineAction::Retire] {
        for mutation in 0..4 {
            let mut next = manual_successor(&previous, action);
            let tracking = next.tracking.as_mut().unwrap();
            match mutation {
                0 => *tracking = capture(&previous),
                1 => tracking.policies.profile = TrackingPolicy::Fixed,
                2 => {
                    tracking.observations.entries[1].evidence_digest =
                        Sha256Digest::from_array([0x77; 32]);
                }
                _ => {
                    tracking.review = TrackingReview::new(
                        DeadlineReviewState::Pending,
                        vec![TrackingReviewRequirement {
                            dependency: TrackingDependency::Source,
                            reason: TrackingReviewReason::DependencyRetired,
                        }],
                    )
                    .unwrap();
                }
            }
            resign(&mut next);
            assert_rejected_link(&previous, &next);
        }
    }
}

#[test]
fn correcting_a_definition_preserves_the_existing_attention() {
    let previous = manual_successor(&accepted(), DeadlineAction::SetAttention);
    let mut next = manual_successor(&previous, DeadlineAction::Correct);
    next.definition.title = label("An explicitly corrected title");
    resign(&mut next);
    assert_successor(&previous, &next);
    assert_eq!(next.attention, previous.attention);
    next.attention = DeadlineAttention::Pending;
    resign(&mut next);
    assert_rejected_link(&previous, &next);
}

#[test]
fn a_technical_legacy_successor_cannot_fabricate_human_tracking_policies() {
    let previous = legacy();
    assert_rejected_link(&previous, &pending(&previous));
}

#[test]
fn evidence_offsets_are_preserved_even_when_the_instant_compares_equal() {
    let previous = accepted();
    for action in [DeadlineAction::Reevaluate, DeadlineAction::SetAttention] {
        let mut next = if action == DeadlineAction::Reevaluate {
            pending(&previous)
        } else {
            manual_successor(&previous, action)
        };
        next.calculation.profile.recorded_at = next
            .calculation
            .profile
            .recorded_at
            .to_offset(time::UtcOffset::from_hms(3, 0, 0).unwrap());
        assert_eq!(next.calculation, previous.calculation);
        resign(&mut next);
        assert_rejected_link(&previous, &next);
    }
}

#[test]
fn legacy_human_history_remains_valid_but_tracked_history_cannot_downgrade() {
    let previous = legacy();
    let mut next = previous.clone();
    next.revision = previous.revision.next().unwrap();
    next.reason = Some(text("Historical attention declaration"));
    next.attention = deadline_support::attention();
    next.receipt.expected_revision = previous.revision.get();
    next.receipt.action = DeadlineAction::SetAttention;
    next.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(700));
    resign(&mut next);
    assert_successor(&previous, &next);
    assert_rejected_link(&accepted(), &next);
}

#[test]
fn human_attention_and_retirement_can_upgrade_legacy_without_declaring_policies() {
    let previous = legacy();
    for action in [DeadlineAction::SetAttention, DeadlineAction::Retire] {
        let mut next = manual_successor(&previous, action);
        let mut tracking = capture(&previous);
        tracking.policies = TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        };
        tracking.review =
            TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![]).unwrap();
        next.tracking = Some(tracking);
        resign(&mut next);
        assert_successor(&previous, &next);
        next.tracking = Some(capture(&previous));
        resign(&mut next);
        assert_rejected_link(&previous, &next);
    }
}

#[test]
fn a_technical_successor_cannot_change_policies_or_erase_pending_reasons() {
    let previous = accepted();
    let mut next = pending(&previous);
    next.tracking.as_mut().unwrap().policies.profile = TrackingPolicy::Fixed;
    resign(&mut next);
    assert_rejected_link(&previous, &next);
    let previous = pending(&previous);
    let mut next = pending(&previous);
    next.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(800));
    let tracking = next.tracking.as_mut().unwrap();
    tracking.observations.entries[1].revision = 3;
    tracking.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![TrackingReviewRequirement {
            dependency: TrackingDependency::Source,
            reason: TrackingReviewReason::DependencyRetired,
        }],
    )
    .unwrap();
    let DeadlineReceiptVersion::Tracked(metadata) = &mut next.receipt.version else {
        unreachable!();
    };
    let Some(TechnicalCause::SourceEvent { event, .. }) = &mut metadata.cause else {
        unreachable!();
    };
    event.revision = 3;
    resign(&mut next);
    assert_rejected_link(&previous, &next);
}

#[test]
fn an_already_observed_event_cannot_create_another_technical_revision() {
    let previous = pending(&accepted());
    let mut next = pending(&previous);
    next.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(800));
    resign(&mut next);
    assert_rejected_link(&previous, &next);
}
