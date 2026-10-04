use crate::review_hearing_support::*;
use application::precautionary_measures::MeasureDecisionAnchorRef;
use time::Duration;
use uuid::Uuid;

fn anchored_imposition() -> (PrecautionaryHearingCapture, MeasureDecisionGroupCapture) {
    let hearing = crate::precautionary_receipt_support::scheduled();
    let mut fixture = MeasureFixture::single();
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.clone(),
    )));
    let group = prepare_measure_decision_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.material,
        &empty_history(),
    )
    .unwrap()
    .into_group_capture(&Hasher, hearing.recorded_at + Duration::seconds(1))
    .unwrap();
    (hearing, group)
}

fn fresh_review(group: &MeasureDecisionGroupCapture) -> ReviewFixture {
    let mut fixture = ReviewFixture::schedule(group);
    fixture.hearing.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(500));
    fixture.hearing.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(501));
    fixture
}

#[test]
fn regression_review_preparation_cannot_reuse_an_ancestor_anchor_hearing_revision() {
    let (anchor, group) = anchored_imposition();
    let mut fixture = fresh_review(&group);
    assert!(fixture.clone().prepare(None).is_ok());
    fixture.hearing.command.hearing_id = anchor.review.command.hearing_id;
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn regression_rehashed_review_receipt_cannot_replace_an_ancestor_anchor_hearing_revision() {
    let (anchor, group) = anchored_imposition();
    let fixture = fresh_review(&group);
    let evidence = fixture.measure_history.clone();
    let mut capture = fixture.capture(None, group.recorded_at + Duration::seconds(1));
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &capture, &evidence)
        .unwrap();
    assert_eq!(
        capture.review.result_revision,
        anchor.review.result_revision
    );
    capture.review.command.hearing_id = anchor.review.command.hearing_id;
    refresh(&mut capture);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher, &capture, &evidence
    )
    .is_err());
}
