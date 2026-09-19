#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;
mod deadline_tracked_support;

use application::{
    deadline_observations::build_legacy_deadline_observations,
    deadline_reevaluation::{ObservationRole, Observations},
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::*,
};
use deadline_observation_support::build;
use deadline_support::evaluation::inputs;
use deadline_technical_support as technical;
use deadline_tracked_support::resign;

fn assert_manual_upgrades_reject_observations(
    previous: &DeadlineDetail,
    observations: &Observations,
) {
    let hasher = inputs::hasher();
    deadline_receipt_matches(hasher.as_ref(), previous).unwrap();
    let historical = build_legacy_deadline_observations(hasher.as_ref(), previous).unwrap();
    assert_ne!(&historical, observations);
    for mut next in [technical::attention(previous), technical::retired(previous)] {
        assert_eq!(next.review_state(), DeadlineReviewState::LegacyUndeclared);
        assert_eq!(next.tracking.as_ref().unwrap().observations, historical);
        deadline_successor_matches(hasher.as_ref(), previous, &next)
            .expect("exact human upgrade must preserve the legacy observations");
        next.tracking.as_mut().unwrap().observations = observations.clone();
        resign(&mut next);
        deadline_receipt_matches(hasher.as_ref(), &next)
            .expect("altered observations must remain coherent with every receipt digest");
        assert_eq!(next.definition, previous.definition);
        assert_eq!(next.calculation, previous.calculation);
        assert_eq!(next.responsible, previous.responsible);
        assert_eq!(
            next.tracking.as_ref().unwrap().administration,
            previous.calculation.material.administration
        );
        let result = deadline_successor_matches(hasher.as_ref(), previous, &next);
        assert!(
            result.is_err(),
            "manual {:?} accepted observations absent from V1: {result:?}",
            next.receipt.action
        );
    }
}

#[test]
fn manual_legacy_upgrade_cannot_advance_to_an_authentic_later_source_head() {
    let previous = technical::legacy();
    let historical =
        build_legacy_deadline_observations(inputs::hasher().as_ref(), &previous).unwrap();
    let later = technical::source_heads(&previous, 2, false);
    let observations = build(&later.profile_head, &later.material, None).unwrap();
    let old_source = historical
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::Source)
        .unwrap();
    let new_source = observations
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::Source)
        .unwrap();
    assert_eq!(old_source.id, new_source.id);
    assert_eq!(old_source.revision, 1);
    assert_eq!(new_source.revision, 2);
    assert_ne!(old_source.submission_digest, new_source.submission_digest);
    assert_ne!(old_source.evidence_digest, new_source.evidence_digest);
    assert_manual_upgrades_reject_observations(&previous, &observations);
}

#[test]
fn manual_legacy_upgrade_cannot_add_an_authentic_unobserved_notification_parent() {
    let mut previous = technical::notification_base(TrackingPolicy::Follow);
    // Encode the same notification calculation in the historical V1 format,
    // which captured the notification but had no separate parent observation.
    previous.tracking = None;
    previous.receipt.version = DeadlineReceiptVersion::Legacy;
    resign(&mut previous);
    deadline_receipt_matches(inputs::hasher().as_ref(), &previous).unwrap();
    let historical =
        build_legacy_deadline_observations(inputs::hasher().as_ref(), &previous).unwrap();
    assert!(historical
        .entries
        .iter()
        .all(|entry| entry.role != ObservationRole::NotificationParent));
    let parent = inputs::resolution(1, false, "2026-01-01");
    let observations = build(
        &previous.calculation.profile,
        &previous.calculation.material,
        Some(&parent),
    )
    .unwrap();
    assert_eq!(observations.entries.len(), historical.entries.len() + 1);
    assert_eq!(
        observations.entries.last().unwrap().role,
        ObservationRole::NotificationParent
    );
    assert_eq!(
        &observations.entries[..historical.entries.len()],
        historical.entries.as_slice()
    );
    assert_manual_upgrades_reject_observations(&previous, &observations);
}
