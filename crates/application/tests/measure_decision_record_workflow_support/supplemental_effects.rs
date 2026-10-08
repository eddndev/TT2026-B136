use super::*;

fn values_input(values: &MeasureValues) -> MeasureValuesInput {
    MeasureValuesInput {
        subject: values.subject(),
        kind: values.kind(),
        conditions: values.conditions().clone(),
        validity: values.validity().clone(),
        supervision: values.supervision().clone(),
    }
}

#[test]
fn workflow_repeated_confirmation_retains_real_m2_then_actual_m2_ancestry() {
    let first = Fixture::corrected().operation(now() - Duration::seconds(1));
    let later = submit(next(&first));
    assert_eq!(later.group.measures[0].result.revision.get(), 4);
    assert_eq!(later.record_history.decisions[0].capture, first.group);
    let OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(previous)) =
        &later.group.review.material.predecessors[0]
    else {
        panic!("expected actual M2")
    };
    assert_eq!(previous.capture, first.group.measures[0]);
    assert_eq!(
        later.group.measures[0].result.values,
        first.group.measures[0].result.values
    );
    assert_eq!(
        later.group.measures[0].result.sources,
        first.group.measures[0].result.sources
    );
}

#[test]
fn workflow_emits_v2_for_standalone_imposition_and_zero_row_no_change() {
    for no_change in [false, true] {
        let pure = if no_change {
            crate::measure_decision_fixtures::Fixture::no_change()
        } else {
            crate::measure_decision_fixtures::Fixture::single()
        };
        let result = submit(from_pure(
            crate::record_decision_support::FixtureV2::initial(pure),
            95,
        ));
        assert_eq!(result.group.measures.len(), usize::from(!no_change));
        assert!(result.record_history.records.judicial.groups.is_empty());
        assert!(result.record_history.records.administrative.is_empty());
        assert!(result.record_history.decisions.is_empty());
        if let Some(row) = result.group.measures.first() {
            assert_eq!(row.result.action, MeasureCaptureAction::Impose);
            assert_eq!(
                row.result.record_root,
                MeasureRecordRoot::Judicial(row.result.judicial_origin)
            );
        }
    }
}

#[test]
fn workflow_modify_revoke_and_cease_use_effective_corrected_values_and_sources() {
    for action in 0..3 {
        let mut fixture = Fixture::corrected();
        let prior = fixture.material.record_history.records.administrative[0]
            .capture
            .clone();
        let previous = crate::record_support::record_reference(&prior.records[0]);
        let effect = match action {
            0 => {
                let mut input = values_input(&prior.review.result.values);
                input.conditions =
                    crate::correction_support::note("Judicially modified effective terms");
                input.supervision = MeasureSupervision::Unknown {
                    reason: crate::correction_support::note("No supervisor declared"),
                };
                fixture.material.result_sources[0].sources.supervisor = None;
                MeasureEffect::Modify {
                    previous,
                    values: MeasureValues::new(input),
                }
            }
            1 => MeasureEffect::Revoke { previous },
            _ => MeasureEffect::Cease { previous },
        };
        fixture.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![effect]))
                .unwrap();
        let result = submit(fixture);
        let row = &result.group.measures[0].result;
        assert_eq!(row.judicial_origin, prior.review.result.judicial_origin);
        assert_eq!(row.record_root, prior.review.result.record_root);
        assert_eq!(row.values.subject(), prior.review.result.values.subject());
        if action == 0 {
            assert_eq!(row.action, MeasureCaptureAction::Modify);
            assert_eq!(
                row.values.conditions().as_str(),
                "Judicially modified effective terms"
            );
            assert!(row.sources.supervisor.is_none());
        } else {
            assert_eq!(
                row.action,
                if action == 1 {
                    MeasureCaptureAction::Revoke
                } else {
                    MeasureCaptureAction::Cease
                }
            );
            assert_eq!(row.values, prior.review.result.values);
            assert_eq!(row.sources, prior.review.result.sources);
        }
    }
}

#[test]
fn workflow_substitution_commits_every_joint_result_and_new_judicial_root() {
    let mut fixture = Fixture::corrected();
    let prior = fixture.material.record_history.records.administrative[0]
        .capture
        .clone();
    let previous = crate::record_support::record_reference(&prior.records[0]);
    let successors: Vec<_> = [80, 90]
        .into_iter()
        .map(|id| MeasureProposal {
            id: MeasureId::from_uuid(Uuid::from_u128(id)),
            values: prior.review.result.values.clone(),
        })
        .collect();
    fixture
        .material
        .result_sources
        .extend(successors.iter().map(|p| MeasureResultSources {
            id: p.id,
            sources: prior.review.result.sources.clone(),
        }));
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Substitute {
                predecessors: vec![previous],
                successors,
            },
        ]))
        .unwrap();
    let stored = submit(fixture);
    assert_eq!(stored.group.measures.len(), 3);
    let outgoing = &stored.group.measures[0];
    assert_eq!(outgoing.result.action, MeasureCaptureAction::SubstituteOut);
    assert_eq!(outgoing.result.values, prior.review.result.values);
    assert_eq!(outgoing.result.sources, prior.review.result.sources);
    for row in &stored.group.measures[1..] {
        assert_eq!(row.result.action, MeasureCaptureAction::SubstituteIn);
        assert_eq!(row.result.revision.get(), 1);
        assert_eq!(
            row.result.judicial_origin.operation_id,
            stored.origin.operation_id
        );
        assert_eq!(
            row.result.record_root,
            MeasureRecordRoot::Judicial(row.result.judicial_origin)
        );
    }
    let relation = &stored.group.substitutions[0];
    assert_eq!(relation.predecessors[0].previous, previous);
    assert_eq!(
        relation.predecessors[0].result,
        crate::record_decision_support::reference_v2(outgoing)
    );
    assert_eq!(relation.successors.len(), 2);
}
