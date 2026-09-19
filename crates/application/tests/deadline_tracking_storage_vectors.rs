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
    cases::CurrentCaseAdministration,
    deadline_tracking::*,
    deadlines::{
        deadline_capture_bytes, deadline_tracking_capture_bytes, decode_deadline_tracking_capture,
    },
};
use deadline_tracking_storage_support::*;
use domain::case_administration::CaseRevision;

#[test]
fn accepted_r0_is_the_unprefixed_102_byte_tracking_suffix() {
    let value = capture();
    let expected = frame(&[2, 2, 0, 1, 0], &value);
    assert_eq!(expected.len(), 102);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    assert_eq!(
        decode_deadline_tracking_capture(&expected)
            .unwrap()
            .administration_revision(),
        None
    );
    roundtrip(&expected, &value);
}

#[test]
fn recorded_administration_has_a_positive_big_endian_revision() {
    for revision in [1, 0x0102_0304, u32::MAX] {
        let mut value = capture();
        recorded(&mut value, revision);
        let expected = frame(&[2, 2, 0, 1, 0], &value);
        assert_eq!(expected.len(), 106);
        assert_eq!(expected[37], 1);
        assert_eq!(&expected[38..42], &revision.to_be_bytes());
        assert_eq!(
            deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
            expected,
        );
        assert_eq!(
            decode_deadline_tracking_capture(&expected)
                .unwrap()
                .administration_revision(),
            Some(CaseRevision::new(revision).unwrap()),
        );
        roundtrip(&expected, &value);
    }
}

#[test]
fn fixed_policy_tags_and_pending_reason_pairs_match_the_manual_vector() {
    let value = pending_mixed();
    let expected = frame(&[2, 1, 0, 2, 3, 0, 1, 1, 2, 2, 3], &value);
    assert_eq!(expected.len(), 108);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    roundtrip(&expected, &value);
}

#[test]
fn each_followed_source_change_uses_the_source_reason_tag() {
    let mut value = capture();
    value.review = TrackingReview::new(
        DeadlineReviewState::Pending,
        vec![requirement(
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged,
        )],
    )
    .unwrap();
    let expected = frame(&[2, 2, 0, 2, 1, 1, 0], &value);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    roundtrip(&expected, &value);
}

#[test]
fn legacy_review_retains_three_undeclared_policies_and_no_reasons() {
    let value = legacy();
    let expected = frame(&[0, 0, 0, 0, 0], &value);
    assert_eq!(expected.len(), 102);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    roundtrip(&expected, &value);
}

#[test]
fn multiple_undeclared_retirements_preserve_every_reason_in_canonical_order() {
    let mut value = with_calendar();
    value.policies = legacy().policies;
    let mut reasons = vec![];
    for dependency in [
        TrackingDependency::Profile,
        TrackingDependency::Source,
        TrackingDependency::Calendar,
    ] {
        reasons.push(requirement(
            dependency,
            TrackingReviewReason::DependencyRetired,
        ));
        reasons.push(requirement(
            dependency,
            TrackingReviewReason::PolicyUndetermined,
        ));
    }
    value.review = TrackingReview::new(DeadlineReviewState::Pending, reasons).unwrap();
    recorded(&mut value, 1);
    let expected = frame(&[0, 0, 0, 2, 6, 0, 2, 0, 3, 1, 2, 1, 3, 2, 2, 2, 3], &value);
    assert_eq!(expected.len(), 118);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    roundtrip(&expected, &value);
}

#[test]
fn codec_bytes_are_exactly_the_existing_dlst2_suffix_for_every_review_state() {
    let accepted = deadline_tracked_support::accepted();
    let pending = deadline_tracked_support::pending(&accepted);
    for mut detail in [accepted, pending] {
        for is_recorded in [false, true] {
            if is_recorded {
                recorded(detail.tracking.as_mut().unwrap(), 1);
            }
            let suffix = deadline_tracking_capture_bytes(
                inputs::hasher().as_ref(),
                detail.tracking.as_ref().unwrap(),
            )
            .unwrap();
            let state = deadline_capture_bytes(inputs::hasher().as_ref(), &detail).unwrap();
            assert_eq!(&state[..5], b"DLST2");
            assert_eq!(&state[state.len() - suffix.len()..], &suffix);
        }
    }
    let mut detail = deadline_tracked_support::accepted();
    detail.tracking = Some(legacy());
    let suffix = deadline_tracking_capture_bytes(
        inputs::hasher().as_ref(),
        detail.tracking.as_ref().unwrap(),
    )
    .unwrap();
    let state = deadline_capture_bytes(inputs::hasher().as_ref(), &detail).unwrap();
    assert_eq!(&state[state.len() - suffix.len()..], &suffix);
}

#[test]
fn accepted_dependencies_can_be_absent_without_fabricating_policies_or_observations() {
    let mut value = capture();
    value.observations.entries.truncate(1);
    value.policies.source = TrackingPolicy::Undetermined;
    let expected = frame(&[2, 0, 0, 1, 0], &value);
    assert_eq!(
        deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
        expected,
    );
    roundtrip(&expected, &value);
}

#[test]
fn full_administrative_capture_preserves_extreme_offsets_and_nanoseconds() {
    for offset in [-93_599, 93_599] {
        let mut value = capture();
        recorded(&mut value, 1);
        let CurrentCaseAdministration::Recorded(snapshot) = &mut value.administration else {
            unreachable!()
        };
        snapshot.changed_at = snapshot
            .changed_at
            .replace_nanosecond(123_456_789)
            .unwrap()
            .to_offset(time::UtcOffset::from_whole_seconds(offset).unwrap());
        let expected = frame(&[2, 2, 0, 1, 0], &value);
        assert_eq!(
            deadline_tracking_capture_bytes(inputs::hasher().as_ref(), &value).unwrap(),
            expected,
        );
        roundtrip(&expected, &value);
    }
}
