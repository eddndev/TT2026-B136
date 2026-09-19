use application::{
    cases::CurrentCaseAdministration,
    deadline_reevaluation::{
        DependencyFamily, ObservationEntry, ObservationRole, Observations, ResolutionReference,
    },
    deadline_tracking::{DeadlineReviewState, TrackingPolicies, TrackingPolicy, TrackingReview},
    deadlines::DeadlineTrackingCapture,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    crypto::Sha256Digest,
};
use uuid::Uuid;

pub(super) fn case() -> CaseId {
    CaseId::from_uuid(Uuid::nil())
}

pub(super) fn digest(byte: u8) -> Sha256Digest {
    // Arbitrary bytes exercise transport shape, not cryptographic validity.
    Sha256Digest::from_bytes(&[byte; 32]).unwrap()
}

pub(super) fn full_observations() -> Observations {
    let profile = ObservationEntry {
        role: ObservationRole::Profile,
        family: DependencyFamily::Profile,
        id: Uuid::nil(),
        revision: 1,
        case_id: None,
        hearing_id: None,
        parent_resolution: None,
        submission_digest: digest(0x11),
        evidence_digest: digest(0x22),
    };
    Observations {
        case_id: case(),
        entries: vec![
            profile.clone(),
            ObservationEntry {
                role: ObservationRole::Source,
                family: DependencyFamily::Notification,
                id: Uuid::from_u128(2),
                revision: 2,
                case_id: Some(case()),
                parent_resolution: Some(ResolutionReference {
                    id: Uuid::from_u128(3),
                    revision: 1,
                }),
                ..profile.clone()
            },
            ObservationEntry {
                role: ObservationRole::Calendar,
                family: DependencyFamily::Calendar,
                id: Uuid::from_u128(4),
                revision: 3,
                ..profile.clone()
            },
            ObservationEntry {
                role: ObservationRole::NotificationParent,
                family: DependencyFamily::Resolution,
                id: Uuid::from_u128(3),
                revision: 4,
                case_id: Some(case()),
                ..profile
            },
        ],
    }
}

pub(super) fn capture() -> DeadlineTrackingCapture {
    DeadlineTrackingCapture {
        policies: TrackingPolicies {
            profile: TrackingPolicy::Follow,
            source: TrackingPolicy::Fixed,
            calendar: TrackingPolicy::Follow,
        },
        review: TrackingReview::new(DeadlineReviewState::Accepted, vec![]).unwrap(),
        observations: full_observations(),
        administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Case", "REF-1").unwrap(),
        ),
    }
}
