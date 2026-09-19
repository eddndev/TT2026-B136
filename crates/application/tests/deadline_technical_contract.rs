#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    cases::CurrentCaseAdministration, deadline_reevaluation::*, deadline_tracking::*, deadlines::*,
};
use deadline_observation_support as observed;
use deadline_technical_support::*;
use domain::case_administration::CaseRevision;
use uuid::Uuid;

#[test]
fn malformed_source_events_are_rejected_before_revision_policy() {
    for kind in 0..5 {
        let base = accepted(TrackingPolicy::Follow);
        let resolved = source_heads(&base, 2, false);
        let mut event = source_event(&resolved, 2);
        match kind {
            0 => event.sequence = 0,
            1 => event.sequence = (i64::MAX as u64) + 1,
            2 => event.revision = 0,
            3 => event.case_id = None,
            _ => event.hearing_id = Some(Uuid::from_u128(700)),
        }
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "{kind}"
        );
    }
}

#[test]
fn the_last_positive_database_event_sequence_is_supported() {
    let base = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&base, 2, false);
    let event = source_event(&resolved, i64::MAX as u64);
    let next = revision(&base, event_command(event), resolved);
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged
    ));
    let DeadlineReceiptVersion::Tracked(metadata) = next.receipt.version else {
        panic!()
    };
    assert_eq!(metadata.cause, Some(event_command(event).cause));
}

#[test]
fn bootstrap_rejects_unsupported_policy_versions() {
    for version in [0, 2, u16::MAX] {
        let base = legacy();
        let mut command = bootstrap();
        command.cause = TechnicalCause::LegacyBootstrap {
            job_id: Uuid::from_u128(900),
            policy_version: version,
        };
        assert!(prepare(&base, command, heads(&base)).is_err(), "{version}");
    }
}

#[test]
fn an_explicitly_fixed_profile_is_observed_but_its_retirement_still_requires_review() {
    for retired in [false, true] {
        let (command, preparation) = deadline_support::fixture();
        let base = human(
            command,
            preparation,
            Some(TrackingPolicies {
                profile: TrackingPolicy::Fixed,
                ..policies(TrackingPolicy::Follow)
            }),
            None,
        );
        let mut resolved = heads(&base);
        observed::replace_profile(&mut resolved.profile_head, retired);
        let next = revision(&base, event_command(profile_event(&resolved, 2)), resolved);
        assert_eq!(next.calculation, base.calculation);
        assert_eq!(next.definition, base.definition);
        assert_eq!(observation(&next, ObservationRole::Profile).revision, 2);
        if retired {
            assert_eq!(next.review_state(), DeadlineReviewState::Pending);
            assert!(has_reason(
                &next,
                TrackingDependency::Profile,
                TrackingReviewReason::DependencyRetired
            ));
            assert_eq!(next.operational_due_at(), None);
        } else {
            assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
            assert_eq!(next.operational_due_at(), base.operational_due_at());
        }
    }
}

#[test]
fn a_fixed_notification_parent_is_preserved_but_cannot_hide_its_withdrawal() {
    for withdrawn in [false, true] {
        let base = notification_base(TrackingPolicy::Fixed);
        let mut resolved = heads(&base);
        let parent = inputs::resolution(2, withdrawn, "2026-01-01");
        let event = fact_event(&parent, 2);
        resolved.notification_parent_head = Some(parent);
        let next = revision(&base, event_command(event), resolved);
        assert_eq!(next.calculation, base.calculation);
        assert_eq!(next.definition, base.definition);
        assert_eq!(
            observation(&next, ObservationRole::Source)
                .parent_resolution
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            observation(&next, ObservationRole::NotificationParent).revision,
            2
        );
        if withdrawn {
            assert_eq!(next.review_state(), DeadlineReviewState::Pending);
            assert!(has_reason(
                &next,
                TrackingDependency::Source,
                TrackingReviewReason::DependencyRetired
            ));
        } else {
            assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
            assert_eq!(next.operational_due_at(), base.operational_due_at());
        }
    }
}

#[test]
fn a_legacy_source_withdrawal_keeps_all_missing_policy_reasons() {
    let model = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let (mut command, mut preparation) = deadline_support::fixture();
    command.change = DeadlineChange::Register {
        definition: model.definition.clone(),
    };
    preparation.resolved = Some(DeadlineResolvedInputs {
        profile: model.calculation.profile.clone(),
        profile_head: model.calculation.profile.clone(),
        material: model.calculation.material.clone(),
        notification_parent_head: None,
    });
    let base = deadline_support::detail(&deadline_support::prepare(command, preparation).unwrap());
    let resolved = source_heads(&base, 2, true);
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.definition, base.definition);
    for dependency in [
        TrackingDependency::Profile,
        TrackingDependency::Source,
        TrackingDependency::Calendar,
    ] {
        assert!(has_reason(
            &next,
            dependency,
            TrackingReviewReason::PolicyUndetermined
        ));
    }
    assert!(has_reason(
        &next,
        TrackingDependency::Source,
        TrackingReviewReason::DependencyRetired
    ));
    assert_eq!(next.tracking.as_ref().unwrap().review.reasons().len(), 4);
}

fn administered_base() -> DeadlineDetail {
    let (command, mut preparation) = deadline_support::fixture();
    preparation.administration = observed::administration(inputs::case_id(), false);
    let CurrentCaseAdministration::Recorded(admin) = &mut preparation.administration else {
        panic!()
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

#[test]
fn technical_observation_cannot_regress_administration_or_rewrite_its_exact_revision() {
    for kind in 0..4 {
        let base = administered_base();
        let mut resolved = source_heads(&base, 2, false);
        let event = source_event(&resolved, 2);
        let CurrentCaseAdministration::Recorded(admin) = &mut resolved.material.administration
        else {
            panic!()
        };
        match kind {
            0 => admin.revision = CaseRevision::FIRST,
            1 => admin.changed_by.email = "changed@example.test".into(),
            2 => {
                admin.changed_at = admin
                    .changed_at
                    .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap())
            }
            _ => {
                resolved.material.administration = observed::administration(base.case_id, true);
                let CurrentCaseAdministration::Recorded(admin) =
                    &mut resolved.material.administration
                else {
                    panic!()
                };
                admin.revision = CaseRevision::new(2).unwrap();
            }
        }
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "{kind}"
        );
    }
}

#[test]
fn a_new_administration_revision_is_captured_outside_the_retained_calculation() {
    let base = administered_base();
    let mut resolved = source_heads(&base, 2, false);
    resolved.material.administration = observed::administration(base.case_id, true);
    let CurrentCaseAdministration::Recorded(admin) = &mut resolved.material.administration else {
        panic!()
    };
    admin.revision = CaseRevision::new(3).unwrap();
    let expected = resolved.material.administration.clone();
    let next = revision(&base, event_command(source_event(&resolved, 2)), resolved);
    assert_eq!(next.tracking.as_ref().unwrap().administration, expected);
    assert_eq!(next.calculation, base.calculation);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
}
