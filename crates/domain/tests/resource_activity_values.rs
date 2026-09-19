use domain::{
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineRevision},
    hearings::{HearingId, HearingRevision},
    procedural_resources::{ResourceActId, ResourceActRevision, ResourceId, ResourceRevision},
    resource_activities::*,
};
use uuid::Uuid;

fn selection() -> ResourceActivitySelection {
    ResourceActivitySelection {
        resource: ResourceCaptureRef {
            id: ResourceId::from_uuid(Uuid::from_u128(1)),
            revision: ResourceRevision::initial(),
            capture_digest: Sha256Digest::from_array([2; 32]),
        },
        act: Some(ResourceActCaptureRef {
            id: ResourceActId::from_uuid(Uuid::from_u128(3)),
            revision: ResourceActRevision::initial(),
            resource_revision: ResourceRevision::new(2).unwrap(),
            capture_digest: Sha256Digest::from_array([4; 32]),
        }),
        target: ResourceActivityTarget::Hearing {
            id: HearingId::from_uuid(Uuid::from_u128(5)),
            revision: HearingRevision::new(3).unwrap(),
            submission_digest: Sha256Digest::from_array([6; 32]),
        },
    }
}

#[test]
fn association_identity_and_revision_are_independent_of_resource_and_activity() {
    let id = ResourceActivityId::from_uuid(Uuid::from_u128(7));
    let operation = ResourceActivityOperationId::from_uuid(Uuid::from_u128(8));
    assert_eq!(id.as_uuid(), Uuid::from_u128(7));
    assert_eq!(operation.as_uuid(), Uuid::from_u128(8));
    assert_eq!(ResourceActivityRevision::initial().get(), 1);
    assert!(ResourceActivityRevision::new(0).is_err());
    assert_eq!(
        ResourceActivityRevision::new(u32::MAX).unwrap().next(),
        None
    );
}

#[test]
fn canonical_selection_frames_every_exact_reference_in_declared_order() {
    let value = selection();
    let mut expected = b"RASL1".to_vec();
    expected.extend_from_slice(Uuid::from_u128(1).as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&[2; 32]);
    expected.push(1);
    expected.extend_from_slice(Uuid::from_u128(3).as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&2_u32.to_be_bytes());
    expected.extend_from_slice(&[4; 32]);
    expected.push(0);
    expected.extend_from_slice(Uuid::from_u128(5).as_bytes());
    expected.extend_from_slice(&3_u32.to_be_bytes());
    expected.extend_from_slice(&[6; 32]);
    assert_eq!(value.canonical_bytes(), expected);
    assert_eq!(value.resource.revision.get(), 1);
    assert_eq!(value.act.unwrap().resource_revision.get(), 2);
}

#[test]
fn target_family_and_optional_act_cannot_collide_in_canonical_selection() {
    let hearing = selection();
    let mut deadline = hearing;
    deadline.target = ResourceActivityTarget::Deadline {
        id: DeadlineId::from_uuid(Uuid::from_u128(5)),
        revision: DeadlineRevision::new(3).unwrap(),
        capture_digest: Sha256Digest::from_array([6; 32]),
    };
    assert_ne!(hearing.canonical_bytes(), deadline.canonical_bytes());
    let mut without_act = hearing;
    without_act.act = None;
    assert_ne!(hearing.canonical_bytes(), without_act.canonical_bytes());
}

#[test]
fn changing_any_historical_capture_changes_the_selection_bytes() {
    let original = selection();
    let mut variants = [original; 5];
    variants[0].resource.revision = ResourceRevision::new(2).unwrap();
    variants[1].resource.capture_digest = Sha256Digest::from_array([9; 32]);
    variants[2].act.as_mut().unwrap().revision = ResourceActRevision::new(2).unwrap();
    variants[3].act.as_mut().unwrap().resource_revision = ResourceRevision::new(3).unwrap();
    variants[4].act.as_mut().unwrap().capture_digest = Sha256Digest::from_array([9; 32]);
    for changed in variants {
        assert_ne!(original.canonical_bytes(), changed.canonical_bytes());
    }
}
