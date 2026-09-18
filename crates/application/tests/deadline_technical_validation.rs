#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use application::{deadline_tracking::TrackingPolicy, procedural_facts::fact_receipt_matches};
use deadline_observation_support as observed;
use deadline_technical_support::*;
use domain::crypto::Sha256Digest;
use time::UtcOffset;

#[test]
fn corrupted_base_receipts_are_rejected_even_for_a_bootstrap_retry() {
    for kind in 0..3 {
        let mut base = accepted(TrackingPolicy::Follow);
        match kind {
            0 => base.receipt.review_digest = Sha256Digest::from_array([0; 32]),
            1 => base.receipt.capture_digest = Sha256Digest::from_array([0; 32]),
            _ => base.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
        }
        assert!(prepare(&base, bootstrap(), heads(&base)).is_err(), "{kind}");
    }
}

#[test]
fn corrupt_current_receipts_and_cross_case_material_are_rejected() {
    for kind in 0..4 {
        let base = accepted(TrackingPolicy::Follow);
        let mut resolved = source_heads(&base, 2, false);
        let event = source_event(&resolved, 2);
        match kind {
            0 => {
                inputs::metadata_mut(observed::fact_mut(&mut resolved.material.source_head))
                    .receipt
                    .submission_digest = Sha256Digest::from_array([0; 32]);
            }
            1 => {
                resolved.profile_head.receipt.submission_digest = Sha256Digest::from_array([0; 32])
            }
            2 => {
                resolved.material.case_id =
                    domain::cases::CaseId::from_uuid(uuid::Uuid::from_u128(999))
            }
            _ => {
                resolved.material.administration = observed::administration(
                    domain::cases::CaseId::from_uuid(uuid::Uuid::from_u128(999)),
                    false,
                );
            }
        }
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "{kind}"
        );
    }
}

#[test]
fn a_future_event_cannot_be_covered_by_an_older_current_head() {
    let base = accepted(TrackingPolicy::Follow);
    let event = fact_event(&inputs::resolution(3, false, "2026-01-10"), 3);
    assert!(prepare(&base, event_command(event), source_heads(&base, 2, false)).is_err());
}

#[test]
fn an_exact_head_event_must_name_its_real_source_operation() {
    let base = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&base, 2, false);
    let mut event = source_event(&resolved, 2);
    event.operation_id = uuid::Uuid::from_u128(999);
    assert!(prepare(&base, event_command(event), resolved).is_err());
}

#[test]
fn a_supplied_selected_source_cannot_replace_immutable_metadata_or_its_offset() {
    for offset_only in [false, true] {
        let base = accepted(TrackingPolicy::Follow);
        let mut resolved = source_heads(&base, 2, false);
        let event = source_event(&resolved, 2);
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
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "{offset_only}"
        );
    }
}

#[test]
fn a_selected_calendar_cannot_change_its_captured_offset_before_recalculation() {
    let base = calendar_base(TrackingPolicy::Follow, TrackingPolicy::Follow);
    let mut resolved = calendar_heads(&base, 2, false);
    let event = calendar_event(resolved.material.calendar_head.as_ref().unwrap(), 2);
    let calendar = resolved.material.calendar.as_mut().unwrap();
    calendar.recorded_at = calendar
        .recorded_at
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    application::judicial_calendars::judicial_calendar_receipt_matches(
        inputs::hasher().as_ref(),
        calendar,
    )
    .unwrap();
    assert_eq!(
        resolved.material.calendar,
        base.calculation.material.calendar
    );
    assert!(prepare(&base, event_command(event), resolved).is_err());
}

#[test]
fn an_unchanged_profile_head_cannot_replace_its_immutable_observed_offset() {
    let base = accepted(TrackingPolicy::Follow);
    let mut resolved = source_heads(&base, 2, false);
    let event = source_event(&resolved, 2);
    resolved.profile_head.recorded_at = resolved
        .profile_head
        .recorded_at
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(resolved.profile_head, base.calculation.profile);
    assert!(prepare(&base, event_command(event), resolved).is_err());
}

#[test]
fn previously_observed_heads_cannot_regress_during_another_dependency_event() {
    let initial = accepted(TrackingPolicy::Follow);
    let resolved = source_heads(&initial, 2, false);
    let base = revision(
        &initial,
        event_command(source_event(&resolved, 2)),
        resolved,
    );
    let mut resolved = heads(&base);
    observed::replace_profile(&mut resolved.profile_head, false);
    let event = profile_event(&resolved, 3);
    assert!(prepare(&base, event_command(event), resolved).is_err());
}

#[test]
fn a_notification_cannot_omit_or_substitute_its_observed_parent_root() {
    for wrong_root in [false, true] {
        let base = notification_base(TrackingPolicy::Follow);
        let mut resolved = heads(&base);
        let parent = inputs::resolution(2, false, "2026-01-01");
        let event = fact_event(&parent, 2);
        if wrong_root {
            let mut other = parent;
            let application::procedural_facts::ProceduralFactSnapshot::Resolution(snapshot) =
                &mut other.snapshot
            else {
                panic!()
            };
            snapshot.root = domain::procedural_facts::ResolutionRoot::new(
                application::procedural_facts::ResolutionId::from_uuid(uuid::Uuid::from_u128(999)),
                base.case_id,
            );
            inputs::resign_fact(&mut other);
            resolved.notification_parent_head = Some(other);
        }
        assert!(
            prepare(&base, event_command(event), resolved).is_err(),
            "{wrong_root}"
        );
    }
}
