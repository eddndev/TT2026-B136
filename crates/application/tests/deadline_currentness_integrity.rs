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
    cases::CurrentCaseAdministration,
    deadline_currentness::{evaluate_deadline_currentness, DeadlineFreshness::Current},
    deadline_inputs::DeadlineSourceDetail,
    deadline_technical::DeadlineReevaluationInputs,
    deadline_tracking::TrackingPolicy,
    deadlines::*,
    procedural_facts::{fact_receipt_matches, ProceduralFactSnapshot, ResolutionId},
    ApplicationError,
};
use deadline_currentness_support::*;
use deadline_observation_support as observed;
use deadline_technical_support::*;
use domain::{case_administration::CaseRevision, crypto::Sha256Digest};
use time::UtcOffset;
use uuid::Uuid;

fn assert_inconsistent(base: &DeadlineDetail, resolved: &DeadlineReevaluationInputs) {
    assert!(matches!(
        evaluate_deadline_currentness(
            inputs::hasher().as_ref(),
            base,
            Some(resolved),
            checked_at()
        ),
        Err(ApplicationError::Deadline(
            DeadlineError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn evaluable_active_records_require_current_inputs() {
    let base = accepted(TrackingPolicy::Follow);
    assert!(
        evaluate_deadline_currentness(inputs::hasher().as_ref(), &base, None, checked_at())
            .is_err()
    );
}

#[test]
fn corrupted_receipts_never_become_an_unchecked_projection() {
    for initial in [
        legacy(),
        attention(&legacy()),
        retired(&accepted(TrackingPolicy::Fixed)),
    ] {
        for kind in 0..3 {
            let mut base = initial.clone();
            let corrupt = Sha256Digest::from_array([0; 32]);
            match kind {
                0 => base.receipt.capture_digest = corrupt,
                1 => base.receipt.review_digest = corrupt,
                _ => base.receipt.submission_digest = corrupt,
            }
            assert!(
                evaluate_deadline_currentness(inputs::hasher().as_ref(), &base, None, checked_at())
                    .is_err(),
                "{kind}"
            );
        }
    }
}

#[test]
fn each_fixed_observed_dependency_rejects_a_regressed_head() {
    let profile = profile_base(TrackingPolicy::Fixed);
    let mut profile_heads = heads(&profile);
    observed::replace_profile(&mut profile_heads.profile_head, false);
    let source = accepted(TrackingPolicy::Fixed);
    let source_inputs = source_heads(&source, 2, false);
    let calendar = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Fixed);
    let calendar_inputs = calendar_heads(&calendar, 2, false);
    let notification = notification_base(TrackingPolicy::Fixed);
    let mut parent_inputs = heads(&notification);
    let parent = inputs::resolution(2, false, "2026-01-01");
    parent_inputs.notification_parent_head = Some(parent.clone());
    for (initial, resolved, event) in [
        (
            profile,
            profile_heads.clone(),
            profile_event(&profile_heads, 2),
        ),
        (
            source,
            source_inputs.clone(),
            source_event(&source_inputs, 2),
        ),
        (
            calendar,
            calendar_inputs.clone(),
            calendar_event(calendar_inputs.material.calendar_head.as_ref().unwrap(), 2),
        ),
        (notification, parent_inputs, fact_event(&parent, 2)),
    ] {
        let base = revision(&initial, event_command(event), resolved);
        let mut regressed = heads(&base);
        if base.calculation.material.source.as_ref().is_some_and(|value| matches!(
            value,
            DeadlineSourceDetail::Fact(fact) if matches!(fact.snapshot, ProceduralFactSnapshot::Notification(_))
        )) {
            regressed.notification_parent_head = Some(inputs::resolution(1, false, "2026-01-01"));
        }
        assert_inconsistent(&base, &regressed);
    }
}

#[test]
fn fixed_observation_equality_includes_metadata_and_exact_timestamp_offset() {
    let initial = accepted(TrackingPolicy::Fixed);
    let exact = source_heads(&initial, 2, false);
    let base = revision(
        &initial,
        event_command(source_event(&exact, 2)),
        exact.clone(),
    );
    for offset_only in [false, true] {
        let mut rewritten = exact.clone();
        let head = observed::fact_mut(&mut rewritten.material.source_head);
        let metadata = inputs::metadata_mut(head);
        if offset_only {
            metadata.recorded_at = metadata
                .recorded_at
                .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
        } else {
            metadata.recorded_by.email = "substituted@example.test".into();
        }
        fact_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
        assert_inconsistent(&base, &rewritten);
    }
}

#[test]
fn fixed_selection_cannot_replace_historical_evidence_even_with_a_valid_receipt() {
    for offset_only in [false, true] {
        let base = accepted(TrackingPolicy::Fixed);
        let mut resolved = source_heads(&base, 2, false);
        let selected = observed::fact_mut(&mut resolved.material.source);
        let metadata = inputs::metadata_mut(selected);
        if offset_only {
            metadata.recorded_at = metadata
                .recorded_at
                .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
        } else {
            metadata.recorded_by.email = "substituted@example.test".into();
        }
        fact_receipt_matches(inputs::hasher().as_ref(), selected).unwrap();
        assert_inconsistent(&base, &resolved);
    }
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Fixed);
    let mut resolved = calendar_heads(&base, 2, false);
    let selected = resolved.material.calendar.as_mut().unwrap();
    selected.recorded_at = selected
        .recorded_at
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(
        resolved.material.calendar,
        base.calculation.material.calendar
    );
    assert_inconsistent(&base, &resolved);
}

#[test]
fn fixed_heads_still_require_valid_receipts_and_the_exact_dependency_identity() {
    for kind in 0..5 {
        let base = accepted(TrackingPolicy::Fixed);
        let mut resolved = source_heads(&base, 2, false);
        match kind {
            0 => {
                inputs::metadata_mut(observed::fact_mut(&mut resolved.material.source_head))
                    .receipt
                    .submission_digest = Sha256Digest::from_array([0; 32])
            }
            1 => {
                resolved.profile_head.receipt.submission_digest = Sha256Digest::from_array([0; 32])
            }
            2 => resolved.material.case_id = domain::cases::CaseId::from_uuid(Uuid::from_u128(999)),
            3 => {
                let head = observed::fact_mut(&mut resolved.material.source_head);
                let ProceduralFactSnapshot::Resolution(snapshot) = &mut head.snapshot else {
                    panic!()
                };
                snapshot.root = domain::procedural_facts::ResolutionRoot::new(
                    ResolutionId::from_uuid(Uuid::from_u128(999)),
                    base.case_id,
                );
                inputs::resign_fact(head);
            }
            _ => resolved.material.source_head = None,
        }
        assert!(
            evaluate_deadline_currentness(
                inputs::hasher().as_ref(),
                &base,
                Some(&resolved),
                checked_at(),
            )
            .is_err(),
            "{kind}"
        );
    }
}

#[test]
fn fixed_profile_equality_preserves_the_captured_offset() {
    let base = profile_base(TrackingPolicy::Fixed);
    let mut resolved = heads(&base);
    resolved.profile_head.recorded_at = resolved
        .profile_head
        .recorded_at
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(resolved.profile_head, base.calculation.profile);
    assert_inconsistent(&base, &resolved);
}

#[test]
fn a_notification_requires_its_exact_independent_parent_and_full_coverage() {
    for kind in 0..4 {
        let base = notification_base(TrackingPolicy::Fixed);
        let mut resolved = heads(&base);
        let mut parent = inputs::resolution(2, false, "2026-01-01");
        if kind == 1 || kind == 2 {
            let ProceduralFactSnapshot::Resolution(snapshot) = &mut parent.snapshot else {
                panic!()
            };
            snapshot.root = domain::procedural_facts::ResolutionRoot::new(
                if kind == 1 {
                    ResolutionId::from_uuid(Uuid::from_u128(999))
                } else {
                    snapshot.root.id()
                },
                if kind == 2 {
                    domain::cases::CaseId::from_uuid(Uuid::from_u128(999))
                } else {
                    base.case_id
                },
            );
            inputs::resign_fact(&mut parent);
        }
        if kind != 0 {
            resolved.notification_parent_head = Some(parent);
        }
        if kind == 3 {
            resolved.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                inputs::notification(2, false, 3, "2026-01-07"),
            )));
        }
        assert!(
            evaluate_deadline_currentness(
                inputs::hasher().as_ref(),
                &base,
                Some(&resolved),
                checked_at(),
            )
            .is_err(),
            "{kind}"
        );
    }
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
        Some(policies(TrackingPolicy::Fixed)),
        None,
    )
}

#[test]
fn administration_may_advance_and_close_without_replacing_the_captured_administration() {
    let base = administered_base();
    let mut resolved = heads(&base);
    resolved.material.administration = observed::administration(base.case_id, true);
    let CurrentCaseAdministration::Recorded(admin) = &mut resolved.material.administration else {
        panic!()
    };
    admin.revision = CaseRevision::new(3).unwrap();
    assert_projection(
        &base,
        Some(&resolved),
        Current,
        &[],
        base.calculation.result.due_at(),
    );
    assert_ne!(
        base.tracking.as_ref().unwrap().administration,
        resolved.material.administration
    );
}

#[test]
fn administration_cannot_regress_disappear_or_rewrite_an_exact_capture() {
    for kind in 0..5 {
        let base = administered_base();
        let mut resolved = heads(&base);
        let CurrentCaseAdministration::Recorded(admin) = &mut resolved.material.administration
        else {
            panic!()
        };
        match kind {
            0 => admin.revision = CaseRevision::FIRST,
            1 => admin.changed_by.email = "substituted@example.test".into(),
            2 => {
                admin.changed_at = admin
                    .changed_at
                    .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap())
            }
            3 => {
                resolved.material.administration = heads(&accepted(TrackingPolicy::Fixed))
                    .material
                    .administration
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
        assert_inconsistent(&base, &resolved);
    }
}
