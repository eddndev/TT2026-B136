#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
use deadline_observation_support::*;

use application::{
    cases::CurrentCaseAdministration, deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::deadline_profile_receipt_matches, deadline_reevaluation::ObservationRole,
    hearing_results::hearing_result_receipt_matches,
    judicial_calendars::judicial_calendar_receipt_matches, procedural_facts::fact_receipt_matches,
};
use domain::{cases::CaseMetadata, crypto::Sha256Digest, hearings::HearingTime};
use time::{Duration, UtcOffset};

#[test]
fn profile_calendar_and_resolution_hashes_use_independent_full_evidence_vectors() {
    let (profile, mut material) = fixture();
    calendar_pair(&mut material);
    let observed = build(&profile, &material, None).unwrap();
    let profile_bytes = vectors::profile(&profile);
    let calendar_bytes = vectors::calendar(material.calendar_head.as_ref().unwrap());
    let fact_bytes = vectors::fact(fact(&material.source_head));
    assert_eq!(
        entry(&observed, ObservationRole::Profile).evidence_digest,
        vectors::digest(4, &profile_bytes)
    );
    assert_eq!(
        entry(&observed, ObservationRole::Calendar).evidence_digest,
        vectors::digest(3, &calendar_bytes)
    );
    assert_eq!(
        entry(&observed, ObservationRole::Source).evidence_digest,
        vectors::digest(0, &fact_bytes)
    );
    assert_eq!(fact_bytes[0], 1);
    assert_ne!(
        vectors::digest(0, &fact_bytes),
        vectors::digest(1, &fact_bytes)
    );
    assert_ne!(
        entry(&observed, ObservationRole::Profile).evidence_digest,
        profile.receipt.submission_digest
    );
    assert_ne!(
        entry(&observed, ObservationRole::Profile).evidence_digest,
        profile.definition_digest
    );
}

#[test]
fn notification_and_related_resolution_use_distinct_family_and_inner_source_tags() {
    let (profile, _) = fixture();
    let (material, parent) = notification_material();
    let observed = build(&profile, &material, Some(&parent)).unwrap();
    let notification_bytes = vectors::fact(fact(&material.source_head));
    let parent_bytes = vectors::fact(&parent);
    assert_eq!(notification_bytes[0], 2);
    assert_eq!(parent_bytes[0], 1);
    assert_eq!(
        entry(&observed, ObservationRole::Source).evidence_digest,
        vectors::digest(1, &notification_bytes)
    );
    assert_eq!(
        entry(&observed, ObservationRole::NotificationParent).evidence_digest,
        vectors::digest(0, &parent_bytes)
    );
}

#[test]
fn profile_metadata_not_bound_by_legacy_receipt_changes_the_observation_commitment() {
    let (profile, material) = fixture();
    let before = digest(&profile, &material, ObservationRole::Profile);
    for kind in 0..3 {
        let mut changed = profile.clone();
        match kind {
            0 => changed.recorded_by.email = "changed@example.test".into(),
            1 => changed.recorded_at += Duration::seconds(1),
            2 => {
                changed.recorded_at = changed
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            }
            _ => unreachable!(),
        }
        assert_eq!(changed.receipt, profile.receipt);
        deadline_profile_receipt_matches(inputs::hasher().as_ref(), &changed).unwrap();
        assert_ne!(
            digest(&changed, &material, ObservationRole::Profile),
            before,
            "{kind}"
        );
    }
}

#[test]
fn calendar_metadata_not_bound_by_legacy_receipt_changes_the_observation_commitment() {
    let (profile, mut material) = fixture();
    calendar_pair(&mut material);
    let before = digest(&profile, &material, ObservationRole::Calendar);
    for kind in 0..3 {
        let mut changed = material.clone();
        let head = changed.calendar_head.as_mut().unwrap();
        match kind {
            0 => head.recorded_by.email = "changed@example.test".into(),
            1 => head.recorded_at += Duration::seconds(1),
            2 => {
                head.recorded_at = head
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            }
            _ => unreachable!(),
        }
        assert_eq!(
            head.receipt,
            material.calendar_head.as_ref().unwrap().receipt
        );
        judicial_calendar_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
        assert_ne!(
            digest(&profile, &changed, ObservationRole::Calendar),
            before,
            "{kind}"
        );
    }
}

#[test]
fn fact_capture_metadata_and_historical_administration_are_committed_in_full() {
    let (profile, mut material) = fixture();
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        false,
        "2026-01-07",
    ))));
    inputs::metadata_mut(fact_mut(&mut material.source_head)).recorded_administration =
        administration(inputs::case_id(), false);
    let before = digest(&profile, &material, ObservationRole::Source);
    for kind in 0..7 {
        let mut changed = material.clone();
        let head = fact_mut(&mut changed.source_head);
        let metadata = inputs::metadata_mut(head);
        match kind {
            0 => metadata.recorded_by.email = "changed@example.test".into(),
            1 => metadata.recorded_at += Duration::seconds(1),
            2 => {
                metadata.recorded_at = metadata
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            }
            3 => {
                metadata.recorded_administration = CurrentCaseAdministration::Unrevised(
                    CaseMetadata::new("Another historical title", "NEW").unwrap(),
                )
            }
            4..=6 => {
                let CurrentCaseAdministration::Recorded(admin) =
                    &mut metadata.recorded_administration
                else {
                    unreachable!()
                };
                match kind {
                    4 => admin.changed_by.email = "changed@example.test".into(),
                    5 => admin.changed_at += Duration::seconds(1),
                    6 => {
                        admin.changed_at = admin
                            .changed_at
                            .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(
            head.snapshot.metadata().receipt,
            fact(&material.source_head).snapshot.metadata().receipt
        );
        fact_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
        assert_ne!(
            digest(&profile, &changed, ObservationRole::Source),
            before,
            "{kind}"
        );
    }
}

#[test]
fn outer_observed_administration_is_validated_but_is_not_dependency_evidence() {
    let (profile, mut material) = fixture();
    let before = build(&profile, &material, None).unwrap();
    material.administration = administration(inputs::case_id(), true);
    assert_eq!(build(&profile, &material, None).unwrap(), before);
}

#[test]
fn hearing_projection_metadata_is_bound_even_when_its_receipt_still_matches() {
    let (profile, _) = fixture();
    let source = hearing::fixture();
    let material = inputs::material(DeadlineSourceDetail::HearingResult(Box::new(
        source.clone(),
    )));
    let before = digest(&profile, &material, ObservationRole::Source);
    for kind in 0..11 {
        let mut changed = source.clone();
        match kind {
            0 => changed.snapshot.recorded_by.email = "changed@example.test".into(),
            1 => changed.snapshot.recorded_at += Duration::seconds(1),
            2 => {
                changed.snapshot.recorded_at = changed
                    .snapshot
                    .recorded_at
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            }
            3 => {
                changed.anchor.scheduled_at =
                    HearingTime::new(changed.anchor.scheduled_at.value() + Duration::seconds(1))
                        .unwrap()
            }
            4 => changed.attendees[0].participant.overview.display_name = "Changed attendee".into(),
            5 => {
                changed.attendees[0].participant.overview.procedural_role =
                    "Changed historical role".into()
            }
            6 => changed.support.as_mut().unwrap().name = "changed.pdf".into(),
            7 => {
                changed.attendees[0].participant.overview.organization =
                    Some("Changed organization".into())
            }
            8 => {
                let digest = Sha256Digest::from_array([123; 32]);
                changed.snapshot.recorded_administration_digest = digest;
                changed.anchor.scheduling_context.administration_digest = digest;
            }
            9 => {
                changed.anchor.scheduling_context.stage_digest =
                    Some(Sha256Digest::from_array([124; 32]))
            }
            10 => {
                changed.attendees[0].participant.values_digest = Sha256Digest::from_array([125; 32])
            }
            _ => unreachable!(),
        }
        assert_eq!(changed.snapshot.receipt, source.snapshot.receipt);
        hearing_result_receipt_matches(inputs::hasher().as_ref(), &changed).unwrap();
        let material = inputs::material(DeadlineSourceDetail::HearingResult(Box::new(changed)));
        assert_ne!(
            digest(&profile, &material, ObservationRole::Source),
            before,
            "{kind}"
        );
    }
}
