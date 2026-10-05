use super::*;
use crate::record_support::{append_administrative, record_reference, RecordFixture};
use application::measure_corrections::{
    prepare_measure_administrative_record_with_history, MeasureAdministrativeAction,
};
use domain::crypto::DocumentHasher;

fn marking_history() -> (
    PrecautionaryMeasureRef,
    MeasureDecisionRecordHistoryEvidence,
) {
    let first = RecordFixture::initial();
    let prior = first.capture();
    let mut next = RecordFixture::next(&prior, &first.history, 1);
    next.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = prepare_measure_administrative_record_with_history(
        &Hasher,
        &next.actor,
        next.case_id,
        next.command,
        next.context,
        &next.history,
    )
    .unwrap()
    .into_capture(&Hasher, next.recorded_at)
    .unwrap();
    (
        record_reference(&marked.records[0]),
        MeasureDecisionRecordHistoryEvidence {
            records: append_administrative(&next.history, &marked),
            decisions: vec![],
        },
    )
}

#[test]
fn schedule_and_replace_reject_the_exact_entered_in_error_record() {
    let prior = Fixture::schedule().operation(at());
    let (target, history) = marking_history();
    for mut fixture in [Fixture::schedule(), Fixture::replace(&prior)] {
        fixture.targets(vec![target]);
        fixture.material.record_history = history.clone();
        let h = harness(fixture.store(), identity(fixture.actor));
        assert!(h
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
    }
}

#[test]
fn coherently_rehashed_replay_cannot_select_an_already_marked_capture() {
    let mut fixture = Fixture::schedule();
    let mut forged = fixture.operation(now());
    let (target, history) = marking_history();
    fixture.targets(vec![target]);
    let PrecautionaryHearingChange::Schedule { values, .. } = &fixture.command.change else {
        unreachable!()
    };
    forged.capture.review.command = fixture.command.clone();
    forged.capture.review.resolved_values = values.clone();
    let review = &mut forged.capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    forged.capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(&forged.capture).unwrap());
    forged.history.captures = vec![forged.capture.clone()];
    forged.history.record_history = history;
    forged.history.origin.submission_digest = forged.capture.review.submission_digest;
    forged.history.origin.review_digest = forged.capture.review.review_digest;
    forged.history.origin.capture_digest = forged.capture.capture_digest;
    let h = harness(fixture.replay_store(forged), identity(fixture.actor));
    assert!(h
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
    assert!(h.events().is_empty());
}
