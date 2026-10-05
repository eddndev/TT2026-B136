use super::*;
use crate::precautionary_receipt_support::at;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn anchored(
    hearing: &PrecautionaryHearingCapture,
    at: OffsetDateTime,
) -> (
    MeasureDecisionGroupCaptureV2,
    MeasureDecisionRecordHistoryEvidence,
) {
    let mut fixture = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::single());
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.clone(),
    )));
    fixture.material.context = hearing.review.observed_context.clone();
    fixture.command.context = expectation(&fixture.material.context);
    fixture.recorded_at = at;
    let group = fixture.capture();
    let history = append_v2(&fixture.history, &group);
    (group, history)
}

#[test]
fn exact_predecessor_capture_cannot_disagree_with_the_same_hearing_anchored_in_g2() {
    let original = HearingFixture::schedule().capture(None, at());
    let mut conflicting = original.clone();
    conflicting.recorded_at += Duration::seconds(1);
    refresh_hearing(&mut conflicting);
    let (group, history) = anchored(&conflicting, conflicting.recorded_at + Duration::seconds(1));
    let replacement =
        DecisionReviewFixture::replace(&original, vec![reference_v2(&group.measures[0])], history);
    assert!(replacement.prepare(Some(&original)).is_err());
}

#[test]
fn new_review_operation_cannot_be_owned_by_a_different_hearing_inside_g2_ancestry() {
    let anchor = HearingFixture::schedule().capture(None, at());
    let (group, history) = anchored(&anchor, anchor.recorded_at + Duration::seconds(1));
    let mut fixture =
        DecisionReviewFixture::schedule(vec![reference_v2(&group.measures[0])], history);
    fixture.hearing.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(999));
    assert_eq!(
        fixture.hearing.command.operation_id,
        anchor.review.command.operation_id
    );
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn direct_hearing_subject_provenance_must_agree_with_the_g2_source_inventory() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let mut fixture = DecisionReviewFixture::schedule(
        vec![reference_v2(&group.measures[0])],
        append_v2(&judicial.history, &group),
    );
    let source = fixture.hearing.sources.participants[1]
        .bound_subject
        .as_mut()
        .unwrap();
    let prior = group.measures[0]
        .result
        .sources
        .supervisor
        .as_ref()
        .unwrap()
        .bound_subject
        .as_ref()
        .unwrap();
    assert_eq!(source.id, prior.id);
    assert_eq!(source.revision, prior.revision);
    source.changed_by.email = "different immutable subject provenance".into();
    assert!(fixture.prepare(None).is_err());
}
