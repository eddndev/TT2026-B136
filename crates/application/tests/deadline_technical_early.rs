#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_reevaluation::TechnicalCause,
    deadline_technical::{
        early_technical_deadline_outcome, DeadlineReevaluationCommand, DeadlineReevaluationNoChange,
    },
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::{deadline_receipt_matches, DeadlineDetail, DeadlineStatus},
};
use deadline_technical_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

fn early(
    base: &DeadlineDetail,
    command: &DeadlineReevaluationCommand,
) -> Result<Option<DeadlineReevaluationNoChange>, application::ApplicationError> {
    early_technical_deadline_outcome(inputs::hasher().as_ref(), base, command)
}

#[test]
fn verified_retirement_is_terminal_without_loading_current_inputs() {
    for base in [
        retired(&legacy()),
        retired(&accepted(TrackingPolicy::Follow)),
        retired(&notification_base(TrackingPolicy::Fixed)),
    ] {
        assert_eq!(base.status, DeadlineStatus::Retired);
        deadline_receipt_matches(inputs::hasher().as_ref(), &base).unwrap();
        let before = base.clone();
        let event = fact_event(&inputs::resolution(2, false, "2026-01-10"), 2);
        for command in [bootstrap(), event_command(event)] {
            assert_eq!(
                early(&base, &command).unwrap(),
                Some(DeadlineReevaluationNoChange::Retired)
            );
        }
        assert_eq!(base, before);
    }
}

#[test]
fn initialized_bootstrap_can_finish_from_its_verified_base_alone() {
    let original = legacy();
    let pending = revision(&original, bootstrap(), heads(&original));
    assert_eq!(pending.review_state(), DeadlineReviewState::Pending);
    for base in [accepted(TrackingPolicy::Follow), pending] {
        assert_eq!(base.status, DeadlineStatus::Active);
        let before = base.clone();
        assert_eq!(
            early(&base, &bootstrap()).unwrap(),
            Some(DeadlineReevaluationNoChange::AlreadyInitialized)
        );
        assert_eq!(base, before);
    }
}

#[test]
fn active_work_requires_inputs_before_being_classified_as_no_change() {
    let original = legacy();
    let undeclared = attention(&original);
    assert_eq!(
        undeclared.review_state(),
        DeadlineReviewState::LegacyUndeclared
    );
    for base in [&original, &undeclared] {
        assert_eq!(early(base, &bootstrap()).unwrap(), None);
    }
    let base = accepted(TrackingPolicy::Follow);
    let observed = fact_event(&inputs::resolution(1, false, "2026-01-06"), 1);
    let mut unrelated = observed;
    unrelated.source_id = Uuid::from_u128(999);
    for event in [observed, unrelated] {
        assert_eq!(early(&base, &event_command(event)).unwrap(), None);
    }
}

#[test]
fn invalid_receipts_cannot_be_hidden_by_a_terminal_or_initialized_base() {
    for original in [
        legacy(),
        accepted(TrackingPolicy::Follow),
        retired(&legacy()),
    ] {
        for corrupt in 0..3 {
            let mut base = original.clone();
            let invalid = Sha256Digest::from_array([0; 32]);
            match corrupt {
                0 => base.receipt.review_digest = invalid,
                1 => base.receipt.capture_digest = invalid,
                _ => base.receipt.submission_digest = invalid,
            }
            assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &base).is_err());
            assert!(
                early(&base, &bootstrap()).is_err(),
                "invalid receipt {corrupt} must precede every early decision"
            );
        }
    }
}

#[test]
fn unsupported_bootstrap_policy_is_rejected_before_any_early_success() {
    for base in [retired(&legacy()), accepted(TrackingPolicy::Follow)] {
        for policy_version in [0, 2, u16::MAX] {
            let command = DeadlineReevaluationCommand {
                operation_id: bootstrap().operation_id,
                cause: TechnicalCause::LegacyBootstrap {
                    job_id: Uuid::from_u128(900),
                    policy_version,
                },
            };
            assert!(early(&base, &command).is_err(), "policy {policy_version}");
        }
    }
}

#[test]
fn retirement_does_not_authorize_malformed_or_cross_case_source_events() {
    let base = retired(&accepted(TrackingPolicy::Follow));
    let valid = fact_event(&inputs::resolution(2, false, "2026-01-10"), 2);
    for corrupt in 0..6 {
        let mut event = valid;
        match corrupt {
            0 => event.sequence = 0,
            1 => event.sequence = (i64::MAX as u64) + 1,
            2 => event.revision = 0,
            3 => event.case_id = None,
            4 => event.case_id = Some(CaseId::from_uuid(Uuid::from_u128(999))),
            _ => event.hearing_id = Some(Uuid::from_u128(700)),
        }
        assert!(
            early(&base, &event_command(event)).is_err(),
            "invalid event {corrupt} must not finish a retired job"
        );
    }
}
