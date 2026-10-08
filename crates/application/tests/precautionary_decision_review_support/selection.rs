use crate::decision_review_support::*;
use crate::measure_decision_fixtures::Fixture;

#[test]
fn terminal_m2_and_its_valid_correction_remain_selectable_review_targets() {
    for kind in 0..3 {
        let mut judicial = judicial_fixture();
        let previous = match &judicial.command.outcome.changes().unwrap()[0] {
            MeasureEffect::Confirm { previous } => *previous,
            _ => panic!("confirmation fixture expected"),
        };
        let prior = &judicial.material.result_sources[0].sources;
        let values = match &judicial.material.predecessors[0] {
            OwnedMeasureRecord::Administrative { capture, .. } => capture.result.values.clone(),
            _ => panic!("administrative predecessor expected"),
        };
        let effect = match kind {
            0 => MeasureEffect::Revoke { previous },
            1 => MeasureEffect::Cease { previous },
            _ => {
                let source = MeasureResultSources {
                    id: id(90),
                    sources: prior.clone(),
                };
                judicial.material.result_sources.push(source);
                MeasureEffect::Substitute {
                    predecessors: vec![previous],
                    successors: vec![MeasureProposal { id: id(90), values }],
                }
            }
        };
        judicial.effects(vec![effect]);
        let group = judicial.capture();
        let history = append_v2(&judicial.history, &group);
        let hearing = DecisionReviewFixture::schedule(
            vec![reference_v2(&group.measures[0])],
            history.clone(),
        )
        .capture(None, group.recorded_at);
        precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &hearing, &history)
            .unwrap();
        let administrative = AdministrativeFixture::after(&group, &judicial.history, 1);
        let correction = administrative.capture();
        let evidence = append_administrative_decision_history(&administrative.history, &correction);
        assert_eq!(
            correction.review.result.last_action,
            group.measures[0].result.action
        );
        let selected = record_reference(&correction.records[0]);
        let hearing = DecisionReviewFixture::schedule(vec![selected], evidence.clone())
            .capture(None, correction.recorded_at);
        assert_eq!(hearing.review.resolved_values.review_targets(), &[selected]);
        precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &hearing, &evidence)
            .unwrap();
    }
}

#[test]
fn one_review_orders_genuine_m2_and_administrative_siblings_without_changing_either() {
    let judicial = FixtureV2::initial(Fixture::multiple());
    let group = judicial.capture();
    let administrative = AdministrativeFixture::after(&group, &judicial.history, 1);
    let correction = administrative.capture();
    let history = append_administrative_decision_history(&administrative.history, &correction);
    let first = record_reference(&correction.records[0]);
    let second = reference_v2(&group.measures[1]);
    let hearing = DecisionReviewFixture::schedule(vec![second, first], history.clone())
        .capture(None, correction.recorded_at);
    assert_eq!(
        hearing.review.resolved_values.review_targets(),
        &[first, second]
    );
    assert_eq!(history.decisions[0].capture, group);
    precautionary_hearing_receipt_with_decision_history_matches(&Hasher, &hearing, &history)
        .unwrap();
}

#[test]
fn two_full_m2_target_lists_preserve_sixty_four_exact_references_in_the_prefix() {
    let mut request = Fixture::single();
    let sources = request.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&sources));
    for value in 71..=101 {
        request.add_imposition(value, values.clone(), sources.clone());
    }
    let first_fixture = FixtureV2::initial(request);
    let first_group = first_fixture.capture();
    let first_history = append_v2(&first_fixture.history, &first_group);
    let first = DecisionReviewFixture::schedule(
        first_group.measures.iter().map(reference_v2).collect(),
        first_history.clone(),
    )
    .capture(None, first_group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &first, &first_history)
            .unwrap();
    let mut next = FixtureV2::next(&first_group, &first_fixture.history, 2);
    next.effects(
        first_group
            .measures
            .iter()
            .map(|row| MeasureEffect::Confirm {
                previous: reference_v2(row),
            })
            .collect(),
    );
    next.material.predecessors = (0..32)
        .rev()
        .map(|index| owned_v2(&first_group, index))
        .collect();
    next.material.result_sources = first_group
        .measures
        .iter()
        .rev()
        .map(|row| MeasureResultSources {
            id: row.result.id,
            sources: row.result.sources.clone(),
        })
        .collect();
    let next_group = next.capture();
    let history = append_v2(&next.history, &next_group);
    let second = DecisionReviewFixture::replace(
        &first,
        next_group.measures.iter().rev().map(reference_v2).collect(),
        history.clone(),
    )
    .capture(Some(&first), next_group.recorded_at);
    assert_eq!(first.review.resolved_values.review_targets().len(), 32);
    assert_eq!(second.review.resolved_values.review_targets().len(), 32);
    assert_ne!(
        first.review.resolved_values.review_targets(),
        second.review.resolved_values.review_targets()
    );
    precautionary_hearing_transition_with_decision_history_matches(
        &Hasher, &first, &second, &history,
    )
    .unwrap();
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &[first, second],
        &origin,
        &history,
    )
    .unwrap();
}
