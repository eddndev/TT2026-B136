use crate::{decision_support::*, measure_history_support::*};
use time::Duration;

fn existing_two_measure_revisions() -> (
    MeasureDecisionGroupCapture,
    MeasureDecisionGroupCapture,
    MeasureHistoryEvidence,
) {
    let first = Fixture::multiple().capture();
    let mut evidence = history(vec![entry(&first, &empty())]);
    let second = confirmation(
        2,
        &[member(&first, 70), member(&first, 80)],
        &evidence,
        at() + Duration::seconds(1),
    );
    append(&mut evidence, &second);
    (first, second, evidence)
}

fn mixed_revision_outcome(
    first: &MeasureDecisionGroupCapture,
    second: &MeasureDecisionGroupCapture,
) -> MeasureDecisionOutcome {
    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Confirm {
            previous: reference(&member(first, 70).capture),
        },
        MeasureEffect::Confirm {
            previous: reference(&member(second, 80).capture),
        },
    ]))
    .unwrap()
}

#[test]
fn regression_preparation_rejects_a_result_revision_already_created_in_ancestor_evidence() {
    let (first, second, evidence) = existing_two_measure_revisions();
    let mut fixture = root_fixture(3, 999);
    fixture.command.outcome = mixed_revision_outcome(&first, &second);
    fixture.material.predecessors = vec![member(&first, 70), member(&second, 80)];
    fixture.material.result_sources = fixture
        .material
        .predecessors
        .iter()
        .map(|item| MeasureResultSources {
            id: item.capture.result.id,
            sources: item.capture.result.sources.clone(),
        })
        .collect();
    let result = prepare_measure_decision_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.material,
        &evidence,
    );
    assert!(
        result.is_err(),
        "measure 70 revision 2 already belongs to the second group"
    );
}

#[test]
fn regression_rehashed_candidate_cannot_recreate_an_existing_measure_revision() {
    let (first, second, evidence) = existing_two_measure_revisions();
    let mut candidate = confirmation(
        3,
        &[member(&second, 70), member(&second, 80)],
        &evidence,
        at() + Duration::seconds(2),
    );
    candidate.review.command.outcome = mixed_revision_outcome(&first, &second);
    candidate.review.material.predecessors = vec![member(&first, 70), member(&second, 80)];
    let previous = reference(&member(&first, 70).capture);
    for result in [
        &mut candidate.review.results[0],
        &mut candidate.measures[0].result,
    ] {
        result.revision = MeasureRevision::new(2).unwrap();
        result.previous = Some(previous);
    }
    refresh_digests(&mut candidate);
    assert!(measure_decision_group_with_history_matches(&Hasher, &candidate, &evidence).is_err());
    assert!(measure_group_origin(&Hasher, &candidate, &evidence).is_err());
}
