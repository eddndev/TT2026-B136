#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;
mod deadline_tracking_storage_support;

use application::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    deadlines::{deadline_tracking_capture_bytes, decode_deadline_tracking_capture},
};
use deadline_tracking_storage_support::*;
use domain::{
    case_administration::{CaseAdministrationValues, CaseRevision},
    cases::{CaseId, CaseMetadata},
    crypto::Sha256Digest,
    identity::UserId,
};
use uuid::Uuid;

#[test]
fn both_administrative_digest_fields_are_verified_independently_for_r0_and_r1() {
    for revision in [0, 1] {
        let mut value = capture();
        if revision > 0 {
            recorded(&mut value, revision);
        }
        let bytes = frame(&[2, 2, 0, 1, 0], &value);
        let start = if revision == 0 { 38 } else { 42 };
        for position in start..start + 64 {
            let mut altered = bytes.clone();
            altered[position] ^= 1;
            assert!(
                decode_deadline_tracking_capture(&altered)
                    .unwrap()
                    .restore(
                        inputs::hasher().as_ref(),
                        value.observations.clone(),
                        value.administration.clone()
                    )
                    .is_err(),
                "administrative digest byte {position}"
            );
        }
    }
}

#[test]
fn restoring_r0_does_not_fabricate_r1_even_when_the_values_are_identical() {
    let original = capture();
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    let mut value = original.clone();
    recorded(&mut value, 1);
    let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
        unreachable!()
    };
    snapshot.values = original.administration.values();
    snapshot.values_digest =
        case_administration_digest(inputs::hasher().as_ref(), &snapshot.values);
    assert_eq!(
        original.administration.values(),
        value.administration.values()
    );
    rejects_restore(&bytes, &value);
    let recorded_bytes = frame(&[2, 2, 0, 1, 0], &value);
    rejects_restore(&recorded_bytes, &original);
}

#[test]
fn unrevised_original_metadata_is_part_of_the_administrative_commitment() {
    let original = capture();
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    let mut value = original.clone();
    value.administration = CurrentCaseAdministration::Unrevised(
        CaseMetadata::new("Changed original metadata", "R0-CHANGED").unwrap(),
    );
    rejects_restore(&bytes, &value);
    assert_ne!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        bytes,
    );
}

#[test]
fn recorded_revision_and_author_metadata_are_bound_beyond_equal_values() {
    let mut original = capture();
    recorded(&mut original, 1);
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    for change in 0..4 {
        let mut value = original.clone();
        let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
            unreachable!()
        };
        match change {
            0 => snapshot.revision = CaseRevision::new(2).unwrap(),
            1 => snapshot.changed_by.id = UserId::from_uuid(Uuid::from_u128(999)),
            2 => snapshot.changed_by.email = "replacement@example.test".into(),
            _ => snapshot.changed_at += time::Duration::seconds(1),
        }
        assert_eq!(
            original.administration.values(),
            value.administration.values()
        );
        rejects_restore(&bytes, &value);
        assert_ne!(
            deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
            bytes
        );
    }
}

#[test]
fn same_instant_with_another_offset_is_not_the_same_administrative_evidence() {
    let mut original = capture();
    recorded(&mut original, 1);
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    for offset in [-93_599, -21_600, 10_800, 93_599] {
        let mut value = original.clone();
        let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
            unreachable!()
        };
        snapshot.changed_at = snapshot
            .changed_at
            .to_offset(time::UtcOffset::from_whole_seconds(offset).unwrap());
        assert_eq!(value.administration, original.administration);
        rejects_restore(&bytes, &value);
        assert_ne!(
            deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
            bytes
        );
    }
}

#[test]
fn nanosecond_changes_are_not_lost_in_the_administrative_evidence() {
    let mut original = capture();
    recorded(&mut original, 1);
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    let mut value = original.clone();
    let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
        unreachable!()
    };
    snapshot.changed_at += time::Duration::nanoseconds(1);
    assert_eq!(
        value
            .administration
            .snapshot()
            .unwrap()
            .changed_at
            .unix_timestamp(),
        original
            .administration
            .snapshot()
            .unwrap()
            .changed_at
            .unix_timestamp()
    );
    rejects_restore(&bytes, &value);
    assert_ne!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        bytes
    );
}

#[test]
fn administrative_values_cannot_be_replaced_even_with_their_new_valid_digest() {
    let mut original = capture();
    recorded(&mut original, 1);
    let bytes = frame(&[2, 2, 0, 1, 0], &original);
    let mut value = original.clone();
    let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
        unreachable!()
    };
    snapshot.values =
        CaseAdministrationValues::basic(CaseMetadata::new("Replacement", "R1-NEW").unwrap());
    snapshot.values_digest =
        case_administration_digest(inputs::hasher().as_ref(), &snapshot.values);
    rejects_restore(&bytes, &value);
}

#[test]
fn a_self_consistent_frame_cannot_hide_a_wrong_case_or_corrupt_values_digest() {
    for wrong_case in [false, true] {
        let mut value = capture();
        recorded(&mut value, 1);
        let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
            unreachable!()
        };
        if wrong_case {
            snapshot.case_id = CaseId::from_uuid(Uuid::from_u128(999));
        } else {
            snapshot.values_digest = Sha256Digest::from_array([99; 32]);
        }
        let bytes = frame(&[2, 2, 0, 1, 0], &value);
        assert!(deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).is_err());
        rejects_restore(&bytes, &value);
    }
}
