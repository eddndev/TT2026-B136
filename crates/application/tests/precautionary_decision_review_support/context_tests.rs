use super::*;
use application::cases::CaseRevision;
use time::Duration;

fn advanced() -> (AdministrativeFixture, MeasureAdministrativeCapture) {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let mut administrative = AdministrativeFixture::after(&group, &judicial.history, 0);
    let mut context = administrative.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at = administrative.recorded_at;
    administrative.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    administrative.command.context = expectation(&administrative.context);
    let capture = administrative.capture();
    (administrative, capture)
}

#[test]
fn review_context_advances_from_c_after_m2_without_substituting_older_judicial_context() {
    let (administrative, prior) = advanced();
    let evidence = append_administrative_decision_history(&administrative.history, &prior);
    let mut original =
        DecisionReviewFixture::schedule(vec![record_reference(&prior.records[0])], evidence);
    set_context(&mut original.hearing, prior.review.context.clone());
    let valid = original.capture(None, prior.recorded_at);
    for mutation in 0..3 {
        let mut fixture = original.clone();
        let mut context = if mutation == 0 {
            crate::context_support::initial()
        } else {
            prior.review.context.material().clone()
        };
        match mutation {
            1 => {
                context.administration.revision = CaseRevision::new(3).unwrap();
                context.administration.changed_at -= Duration::seconds(1);
            }
            2 => {
                context.administration.changed_by.email =
                    "contradictory captured administrator".into()
            }
            _ => {}
        }
        let changed = PrecautionaryContext::new(&Hasher, context).unwrap();
        set_context(&mut fixture.hearing, changed.clone());
        assert!(fixture.prepare(None).is_err());
        let mut forged = valid.clone();
        forged.review.command = fixture.hearing.command;
        forged.review.scheduling_context = changed.clone();
        forged.review.observed_context = changed;
        refresh_hearing(&mut forged);
        assert!(precautionary_hearing_receipt_with_decision_history_matches(
            &Hasher,
            &forged,
            &fixture.decision_history
        )
        .is_err());
    }
}

#[test]
fn review_capture_and_predecessor_must_not_predate_effective_c_after_m2() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let administrative = AdministrativeFixture::after(&group, &judicial.history, 0);
    let prior = administrative.capture();
    let evidence = append_administrative_decision_history(&administrative.history, &prior);
    let fixture = DecisionReviewFixture::schedule(
        vec![record_reference(&prior.records[0])],
        evidence.clone(),
    );
    let before = prior.recorded_at - Duration::nanoseconds(1);
    assert!(before > group.recorded_at);
    assert!(fixture
        .prepare(None)
        .unwrap()
        .into_capture(&Hasher, before)
        .is_err());
    let mut forged = fixture.capture(None, prior.recorded_at);
    forged.recorded_at = before;
    refresh_hearing(&mut forged);
    assert!(precautionary_hearing_receipt_with_decision_history_matches(
        &Hasher, &forged, &evidence
    )
    .is_err());
    assert!(DecisionReviewFixture::cancel(&forged, evidence.clone())
        .prepare(Some(&forged))
        .is_err());
    assert!(DecisionReviewFixture::replace(
        &forged,
        vec![record_reference(&prior.records[0])],
        evidence
    )
    .prepare(Some(&forged))
    .is_err());
}

#[test]
fn rehashed_replacement_selecting_a_marked_c_after_m2_rejects_transition_and_prefix() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let evidence = append_v2(&judicial.history, &group);
    let fixture =
        DecisionReviewFixture::schedule(vec![reference_v2(&group.measures[0])], evidence.clone());
    let original = fixture.capture(None, group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &original, &evidence).unwrap();
    let mut marking = AdministrativeFixture::after(&group, &judicial.history, 0);
    marking.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = marking.capture();
    let mut replacement =
        DecisionReviewFixture::replace(&original, vec![reference_v2(&group.measures[0])], evidence)
            .capture(Some(&original), marked.recorded_at);
    retarget(&mut replacement, record_reference(&marked.records[0]));
    let history = append_administrative_decision_history(&marking.history, &marked);
    assert!(
        precautionary_hearing_transition_with_decision_history_matches(
            &Hasher,
            &original,
            &replacement,
            &history
        )
        .is_err()
    );
    assert!(precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &[original, replacement],
        &origin,
        &history
    )
    .is_err());
}
