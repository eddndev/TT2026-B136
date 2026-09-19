#![allow(dead_code)]
pub use crate::deadline_support::evaluation::inputs;
use application::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    deadline_reevaluation::{encode_observations, DependencyFamily, ObservationRole},
    deadline_tracking::*,
    deadlines::{
        deadline_tracking_capture_bytes, decode_deadline_tracking_capture, DeadlineTrackingCapture,
    },
};
use domain::{case_administration::CaseRevision, crypto::Sha256Digest};

pub fn capture() -> DeadlineTrackingCapture {
    crate::deadline_tracked_support::accepted()
        .tracking
        .unwrap()
}

pub fn recorded(value: &mut DeadlineTrackingCapture, revision: u32) {
    value.administration =
        crate::deadline_observation_support::administration(value.observations.case_id, false);
    let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
        unreachable!()
    };
    snapshot.revision = CaseRevision::new(revision).unwrap();
}

pub fn with_calendar() -> DeadlineTrackingCapture {
    let mut value = capture();
    let mut calendar = value.observations.entries[0].clone();
    calendar.role = ObservationRole::Calendar;
    calendar.family = DependencyFamily::Calendar;
    calendar.id = uuid::Uuid::from_u128(80);
    calendar.case_id = None;
    value.observations.entries.push(calendar);
    value.policies.calendar = TrackingPolicy::Follow;
    value
}

pub fn requirement(
    dependency: TrackingDependency,
    reason: TrackingReviewReason,
) -> TrackingReviewRequirement {
    TrackingReviewRequirement { dependency, reason }
}

pub fn pending_mixed() -> DeadlineTrackingCapture {
    let mut value = with_calendar();
    value.policies.source = TrackingPolicy::Fixed;
    value.policies.calendar = TrackingPolicy::Undetermined;
    value.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![
            requirement(
                TrackingDependency::Profile,
                TrackingReviewReason::ProfileChanged,
            ),
            requirement(
                TrackingDependency::Source,
                TrackingReviewReason::DependencyRetired,
            ),
            requirement(
                TrackingDependency::Calendar,
                TrackingReviewReason::PolicyUndetermined,
            ),
        ],
    )
    .unwrap();
    value
}

pub fn legacy() -> DeadlineTrackingCapture {
    let mut value = capture();
    value.policies = TrackingPolicies {
        profile: TrackingPolicy::Undetermined,
        source: TrackingPolicy::Undetermined,
        calendar: TrackingPolicy::Undetermined,
    };
    value.review = TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![]).unwrap();
    value
}

// Assemble the documented administrative evidence independently of the writer.
pub fn administrative_evidence(value: &CurrentCaseAdministration) -> Vec<u8> {
    let digest = case_administration_digest(inputs::hasher().as_ref(), &value.values());
    let mut bytes = digest.as_bytes().to_vec();
    match value {
        CurrentCaseAdministration::Unrevised(_) => bytes.push(0),
        CurrentCaseAdministration::Recorded(snapshot) => {
            bytes.push(1);
            bytes.extend(snapshot.case_id.as_uuid().as_bytes());
            bytes.extend(snapshot.revision.get().to_be_bytes());
            bytes.extend(snapshot.values_digest.as_bytes());
            bytes.extend(snapshot.changed_by.id.as_uuid().as_bytes());
            bytes.extend((snapshot.changed_by.email.len() as u64).to_be_bytes());
            bytes.extend(snapshot.changed_by.email.as_bytes());
            bytes.extend(snapshot.changed_at.unix_timestamp_nanos().to_be_bytes());
            bytes.extend(snapshot.changed_at.offset().whole_seconds().to_be_bytes());
        }
    }
    bytes
}

// Header tags and reason pairs are literal vectors supplied by each test.
pub fn frame(header: &[u8], value: &DeadlineTrackingCapture) -> Vec<u8> {
    let hasher = inputs::hasher();
    let observations = encode_observations(&value.observations).unwrap();
    frame_with_digest(header, value, hasher.hash_bytes(&observations))
}

pub fn frame_with_digest(
    header: &[u8],
    value: &DeadlineTrackingCapture,
    observations_digest: Sha256Digest,
) -> Vec<u8> {
    let hasher = inputs::hasher();
    let mut bytes = header.to_vec();
    bytes.extend(observations_digest.as_bytes());
    match value.administration.revision() {
        None => bytes.push(0),
        Some(revision) => {
            bytes.push(1);
            bytes.extend(revision.get().to_be_bytes());
        }
    }
    bytes.extend(
        case_administration_digest(hasher.as_ref(), &value.administration.values()).as_bytes(),
    );
    bytes.extend(
        hasher
            .hash_bytes(&administrative_evidence(&value.administration))
            .as_bytes(),
    );
    bytes
}

pub fn roundtrip(bytes: &[u8], value: &DeadlineTrackingCapture) {
    let metadata = decode_deadline_tracking_capture(bytes).unwrap();
    assert_eq!(
        metadata.administration_revision(),
        value.administration.revision()
    );
    let restored = metadata
        .restore(
            inputs::hasher().as_ref(),
            value.observations.clone(),
            value.administration.clone(),
        )
        .unwrap();
    assert_eq!(&restored, value);
    if let Some(expected) = value.administration.snapshot() {
        let actual = restored.administration.snapshot().unwrap();
        assert_eq!(actual.changed_at.offset(), expected.changed_at.offset());
        assert_eq!(
            actual.changed_at.unix_timestamp_nanos(),
            expected.changed_at.unix_timestamp_nanos()
        );
    }
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &restored).unwrap(),
        bytes,
    );
}

pub fn rejects_restore(bytes: &[u8], value: &DeadlineTrackingCapture) {
    if let Ok(metadata) = decode_deadline_tracking_capture(bytes) {
        assert!(metadata
            .restore(
                inputs::hasher().as_ref(),
                value.observations.clone(),
                value.administration.clone(),
            )
            .is_err());
    }
}
