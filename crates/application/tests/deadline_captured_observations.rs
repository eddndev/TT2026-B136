#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;

use application::{
    deadline_observations::{build_deadline_observations, verify_captured_deadline_observations},
    deadline_reevaluation::ObservationRole,
};
use deadline_observation_support::{fixture, inputs, notification_material};
use domain::crypto::Sha256Digest;
use time::UtcOffset;

#[test]
fn exact_observed_material_verifies_without_recalculating_or_reading_current_heads() {
    let (profile, material) = fixture();
    let hasher = inputs::hasher();
    let observations =
        build_deadline_observations(hasher.as_ref(), material.case_id, &profile, &material, None)
            .unwrap();
    verify_captured_deadline_observations(
        hasher.as_ref(),
        &observations,
        &profile,
        &material,
        None,
    )
    .unwrap();
}

#[test]
fn historical_notification_does_not_invent_an_unobserved_parent() {
    let (profile, _) = fixture();
    let (material, parent) = notification_material();
    let hasher = inputs::hasher();
    let complete = build_deadline_observations(
        hasher.as_ref(),
        material.case_id,
        &profile,
        &material,
        Some(&parent),
    )
    .unwrap();
    verify_captured_deadline_observations(
        hasher.as_ref(),
        &complete,
        &profile,
        &material,
        Some(&parent),
    )
    .unwrap();
    assert!(verify_captured_deadline_observations(
        hasher.as_ref(),
        &complete,
        &profile,
        &material,
        None,
    )
    .is_err());
    let mut partial = complete;
    partial
        .entries
        .retain(|entry| entry.role != ObservationRole::NotificationParent);
    verify_captured_deadline_observations(hasher.as_ref(), &partial, &profile, &material, None)
        .unwrap();
    assert!(verify_captured_deadline_observations(
        hasher.as_ref(),
        &partial,
        &profile,
        &material,
        Some(&parent),
    )
    .is_err());
    assert!(build_deadline_observations(
        hasher.as_ref(),
        material.case_id,
        &profile,
        &material,
        None,
    )
    .is_err());
}

#[test]
fn a_valid_manifest_cannot_replace_receipts_evidence_or_the_observed_entry_set() {
    let (profile, material) = fixture();
    let hasher = inputs::hasher();
    let original =
        build_deadline_observations(hasher.as_ref(), material.case_id, &profile, &material, None)
            .unwrap();
    for change in 0..4 {
        let mut observations = original.clone();
        match change {
            0 => observations.entries[0].evidence_digest = Sha256Digest::from_array([0; 32]),
            1 => observations.entries[0].submission_digest = Sha256Digest::from_array([0; 32]),
            2 => observations.entries[0].revision += 1,
            _ => {
                observations
                    .entries
                    .retain(|entry| entry.role == ObservationRole::Profile);
            }
        }
        assert!(
            verify_captured_deadline_observations(
                hasher.as_ref(),
                &observations,
                &profile,
                &material,
                None,
            )
            .is_err(),
            "change {change}"
        );
    }
}

#[test]
fn captured_metadata_keeps_the_original_offset_even_for_the_same_instant() {
    let (mut profile, material) = fixture();
    let hasher = inputs::hasher();
    let observations =
        build_deadline_observations(hasher.as_ref(), material.case_id, &profile, &material, None)
            .unwrap();
    profile.recorded_at = profile
        .recorded_at
        .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
    assert!(verify_captured_deadline_observations(
        hasher.as_ref(),
        &observations,
        &profile,
        &material,
        None,
    )
    .is_err());
}
