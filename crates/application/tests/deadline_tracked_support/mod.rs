#![allow(dead_code)]
use crate::deadline_support::{
    evaluation::{inputs, text},
    *,
};
use application::{
    deadline_inputs::DeadlineSourceDetail, deadline_reevaluation::*, deadline_tracking::*,
    deadlines::*,
};
use domain::crypto::Sha256Digest;
use uuid::Uuid;

pub fn legacy() -> DeadlineDetail {
    let (command, preparation) = fixture();
    detail(&prepare(command, preparation).unwrap())
}
pub fn capture(value: &DeadlineDetail) -> DeadlineTrackingCapture {
    let profile = &value.calculation.profile;
    let source = match value.calculation.material.source.as_ref().unwrap() {
        DeadlineSourceDetail::Fact(value) => value,
        _ => unreachable!(),
    };
    DeadlineTrackingCapture {
        policies: TrackingPolicies {
            profile: TrackingPolicy::Follow,
            source: TrackingPolicy::Follow,
            calendar: TrackingPolicy::Undetermined,
        },
        review: TrackingReview::new(DeadlineReviewState::Accepted, vec![]).unwrap(),
        observations: Observations {
            case_id: value.case_id,
            entries: vec![
                ObservationEntry {
                    role: ObservationRole::Profile,
                    family: DependencyFamily::Profile,
                    id: profile.id.as_uuid(),
                    revision: profile.revision.get(),
                    case_id: Some(value.case_id),
                    hearing_id: None,
                    parent_resolution: None,
                    submission_digest: profile.receipt.submission_digest,
                    evidence_digest: Sha256Digest::from_array([3; 32]),
                },
                ObservationEntry {
                    role: ObservationRole::Source,
                    family: DependencyFamily::Resolution,
                    id: Uuid::from_u128(10),
                    revision: source.snapshot.metadata().revision.get(),
                    case_id: Some(value.case_id),
                    hearing_id: None,
                    parent_resolution: None,
                    submission_digest: source.snapshot.metadata().receipt.submission_digest,
                    evidence_digest: Sha256Digest::from_array([4; 32]),
                },
            ],
        },
        administration: value.calculation.material.administration.clone(),
    }
}
pub fn resign(value: &mut DeadlineDetail) {
    let hasher = inputs::hasher();
    if let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version {
        metadata.observations_digest = hasher.hash_bytes(
            &encode_observations(&value.tracking.as_ref().unwrap().observations).unwrap(),
        );
    }
    value.receipt.review_digest =
        hasher.hash_bytes(&deadline_review_bytes(hasher.as_ref(), value).unwrap());
    value.receipt.capture_digest =
        hasher.hash_bytes(&deadline_capture_bytes(hasher.as_ref(), value).unwrap());
    value.receipt.submission_digest =
        hasher.hash_bytes(&deadline_record_submission_bytes(value).unwrap());
}
pub fn accepted() -> DeadlineDetail {
    let mut value = legacy();
    value.tracking = Some(capture(&value));
    value.receipt.version = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: Sha256Digest::from_array([0; 32]),
        predecessor: None,
        cause: None,
    });
    resign(&mut value);
    value
}
pub fn pending(base: &DeadlineDetail) -> DeadlineDetail {
    let mut value = base.clone();
    let mut tracking = capture(base);
    tracking.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![TrackingReviewRequirement {
            dependency: TrackingDependency::Source,
            reason: TrackingReviewReason::SourceChanged,
        }],
    )
    .unwrap();
    tracking.observations.entries[1].revision = 2;
    value.tracking = Some(tracking);
    value.revision = base.revision.next().unwrap();
    value.reason = Some(text("Source changed; review qualification"));
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(102));
    value.receipt.expected_revision = base.revision.get();
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    value.receipt.version = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: Sha256Digest::from_array([0; 32]),
        predecessor: Some(PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        }),
        cause: Some(TechnicalCause::SourceEvent {
            job_id: Uuid::from_u128(500),
            event: SourceEventReference {
                sequence: 1,
                family: DependencyFamily::Resolution,
                source_id: Uuid::from_u128(10),
                revision: 2,
                case_id: Some(base.case_id),
                hearing_id: None,
                operation_id: Uuid::from_u128(600),
            },
        }),
    });
    resign(&mut value);
    value
}
