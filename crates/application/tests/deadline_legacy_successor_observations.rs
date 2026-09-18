#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;

use application::{
    deadline_inputs::{DeadlineInputMaterial, DeadlineSourceDetail},
    deadline_reevaluation::*,
    deadline_tracking::*,
    deadlines::*,
};
use deadline_observation_support::{build, fact_mut};
use deadline_support::evaluation::{inputs, text};
use deadline_tracked_support::resign;
use domain::crypto::Sha256Digest;
use uuid::Uuid;

fn legacy(observed_revision: u32) -> DeadlineDetail {
    let (command, mut preparation) = deadline_support::fixture();
    if observed_revision > 1 {
        preparation.resolved.as_mut().unwrap().material.source_head =
            Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
                observed_revision,
                false,
                "2026-01-10",
            ))));
    }
    deadline_support::detail(&deadline_support::prepare(command, preparation).unwrap())
}
fn heads(base: &DeadlineDetail, revision: u32) -> DeadlineInputMaterial {
    let mut material = base.calculation.material.clone();
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        revision,
        false,
        if revision == 1 {
            "2026-01-06"
        } else {
            "2026-01-10"
        },
    ))));
    material
}
fn event(revision: u32) -> TechnicalCause {
    let detail = inputs::resolution(
        revision,
        false,
        if revision == 1 {
            "2026-01-06"
        } else {
            "2026-01-10"
        },
    );
    TechnicalCause::SourceEvent {
        job_id: Uuid::from_u128(900),
        event: SourceEventReference {
            sequence: u64::from(revision),
            family: DependencyFamily::Resolution,
            source_id: Uuid::from_u128(10),
            revision,
            case_id: Some(detail.snapshot.case_id()),
            hearing_id: None,
            operation_id: detail.snapshot.metadata().receipt.operation_id.as_uuid(),
        },
    }
}
fn bootstrap() -> TechnicalCause {
    TechnicalCause::LegacyBootstrap {
        job_id: Uuid::from_u128(901),
        policy_version: 1,
    }
}
fn successor(
    base: &DeadlineDetail,
    material: &DeadlineInputMaterial,
    cause: TechnicalCause,
) -> DeadlineDetail {
    let mut value = base.clone();
    value.revision = base.revision.next().unwrap();
    value.reason = Some(text(
        "Legacy capture requires explicit human tracking policies",
    ));
    value.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    value.receipt.action = DeadlineAction::Reevaluate;
    value.receipt.expected_revision = base.revision.get();
    value.receipt.operation_id = DeadlineOperationId::from_uuid(Uuid::from_u128(902));
    value.receipt.version = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: Sha256Digest::from_array([0; 32]),
        predecessor: Some(PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        }),
        cause: Some(cause),
    });
    value.tracking = Some(DeadlineTrackingCapture {
        policies: TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        },
        review: TrackingReview::new(
            DeadlineReviewState::Pending,
            [TrackingDependency::Profile, TrackingDependency::Source]
                .map(|dependency| TrackingReviewRequirement {
                    dependency,
                    reason: TrackingReviewReason::PolicyUndetermined,
                })
                .to_vec(),
        )
        .unwrap(),
        observations: build(&base.calculation.profile, material, None).unwrap(),
        administration: base.calculation.material.administration.clone(),
    });
    resign(&mut value);
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
fn assert_link(previous: &DeadlineDetail, next: &DeadlineDetail, allowed: bool) {
    deadline_receipt_matches(inputs::hasher().as_ref(), previous).unwrap();
    deadline_receipt_matches(inputs::hasher().as_ref(), next).unwrap();
    assert_eq!(next.calculation, previous.calculation);
    let result = deadline_successor_matches(inputs::hasher().as_ref(), previous, next);
    assert_eq!(
        result.is_ok(),
        allowed,
        "unexpected successor decision: {result:?}"
    );
}

#[test]
fn legacy_captured_heads_already_observe_earlier_or_equal_source_events() {
    for (previous_revision, event_revision, new_head) in [(1, 1, 3), (3, 2, 5), (3, 3, 5)] {
        let previous = legacy(previous_revision);
        let next = successor(
            &previous,
            &heads(&previous, new_head),
            event(event_revision),
        );
        assert_link(&previous, &next, false);
    }
}

#[test]
fn a_legacy_upgrade_cannot_regress_an_already_captured_head() {
    let previous = legacy(3);
    for cause in [event(2), bootstrap()] {
        let next = successor(&previous, &heads(&previous, 2), cause);
        assert_link(&previous, &next, false);
    }
}

#[test]
fn a_legacy_upgrade_cannot_replace_evidence_of_the_same_observed_revision() {
    let previous = legacy(3);
    let mut material = heads(&previous, 3);
    inputs::metadata_mut(fact_mut(&mut material.source_head)).recorded_at = previous
        .calculation
        .material
        .source_head
        .as_ref()
        .and_then(|source| match source {
            DeadlineSourceDetail::Fact(value) => Some(value.snapshot.metadata().recorded_at),
            _ => None,
        })
        .unwrap()
        .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    let next = successor(&previous, &material, bootstrap());
    assert_link(&previous, &next, false);
}

#[test]
fn legacy_bootstrap_and_unobserved_coalesced_events_preserve_historical_heads() {
    let previous = legacy(3);
    for (cause, revision) in [(bootstrap(), 3), (event(4), 5)] {
        let next = successor(&previous, &heads(&previous, revision), cause);
        assert_link(&previous, &next, true);
        assert_eq!(next.review_state(), DeadlineReviewState::Pending);
        assert!(next.operational_due_at().is_none());
    }
}
