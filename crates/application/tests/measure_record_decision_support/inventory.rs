use crate::measure_decision_fixtures::Fixture;
use crate::record_decision_support::*;

#[test]
fn thirty_two_v2_predecessors_and_results_normalize_without_truncation() {
    let mut request = Fixture::single();
    let MeasureEffect::Impose(first) = &request.command.outcome.changes().unwrap()[0] else {
        panic!("imposition expected")
    };
    let values = first.values.clone();
    let sources = request.material.result_sources[0].sources.clone();
    for value in 71..102 {
        request.add_imposition(value, values.clone(), sources.clone());
    }
    let initial_fixture = FixtureV2::initial(request);
    let initial = initial_fixture.capture();
    let mut next = FixtureV2::next(&initial, &initial_fixture.history, 2);
    next.effects(
        initial
            .measures
            .iter()
            .rev()
            .map(|row| MeasureEffect::Confirm {
                previous: reference_v2(row),
            })
            .collect(),
    );
    next.material.predecessors = (0..32)
        .rev()
        .map(|index| owned_v2(&initial, index))
        .collect();
    next.material.result_sources = initial
        .measures
        .iter()
        .rev()
        .map(|row| MeasureResultSources {
            id: row.result.id,
            sources: row.result.sources.clone(),
        })
        .collect();
    let reversed = next.capture();
    next.material.predecessors.reverse();
    next.material.result_sources.reverse();
    let ordered = next.capture();
    assert_eq!(reversed, ordered);
    assert_eq!(ordered.measures.len(), 32);
    assert_eq!(ordered.review.material.predecessors.len(), 32);
    let selected: Vec<_> = ordered.measures.iter().rev().map(reference_v2).collect();
    let evidence = append_v2(&next.history, &ordered);
    let checked =
        resolve_measure_records_with_decision_history(&Hasher, next.case_id, &selected, &evidence)
            .unwrap();
    assert_eq!(checked.targets().len(), 32);
    for (offset, record) in checked.targets().iter().enumerate() {
        assert_eq!(record.reference().id(), id(70 + offset as u128));
        assert_eq!(record.reference().revision().get(), 2);
    }
}

#[test]
fn substitution_can_join_two_corrected_predecessors_without_pairing_successors() {
    let initial = Fixture::multiple().capture();
    let first_fixture = RecordFixture::from_first(CorrectionFixture::from_group(
        &initial,
        &crate::effect_support::empty_history(),
        id(70),
    ));
    let mut second_fixture = RecordFixture::from_first(CorrectionFixture::from_group(
        &initial,
        &crate::effect_support::empty_history(),
        id(80),
    ));
    second_fixture.command.operation_id =
        domain::precautionary_measures::MeasureCorrectionOperationId::from_uuid(
            uuid::Uuid::from_u128(501),
        );
    let first = first_fixture.capture();
    let second = second_fixture.capture();
    let mut next = FixtureV2::confirm(&first, &first_fixture.history);
    let second_origin =
        measure_administrative_origin_with_history(&Hasher, &second, &second_fixture.history)
            .unwrap();
    next.history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin: second_origin,
            capture: second.clone(),
        });
    next.material
        .predecessors
        .push(OwnedMeasureRecord::Administrative {
            owner: MeasureAdministrativeRef {
                operation_id: second.review.command.operation_id,
                capture_digest: second.capture_digest,
            },
            capture: Box::new(second.records[0].clone()),
        });
    next.effects(vec![MeasureEffect::Substitute {
        predecessors: vec![
            record_reference(&second.records[0]),
            record_reference(&first.records[0]),
        ],
        successors: [90, 100]
            .into_iter()
            .map(|value| MeasureProposal {
                id: id(value),
                values: first.review.result.values.clone(),
            })
            .collect(),
    }]);
    next.material.result_sources.push(MeasureResultSources {
        id: id(80),
        sources: second.review.result.sources.clone(),
    });
    next.material
        .result_sources
        .extend([90, 100].into_iter().map(|value| MeasureResultSources {
            id: id(value),
            sources: first.review.result.sources.clone(),
        }));
    let group = next.capture();
    assert_eq!(group.substitutions.len(), 1);
    assert_eq!(group.substitutions[0].predecessors.len(), 2);
    assert_eq!(group.substitutions[0].successors.len(), 2);
    assert_eq!(group.measures.len(), 4);
    measure_decision_group_v2_matches(&Hasher, &group, &next.history).unwrap();
}
