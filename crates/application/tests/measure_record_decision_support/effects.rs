use crate::effect_support::values_input;
use crate::measure_decision_fixtures::Fixture;
use crate::record_decision_support::*;

#[test]
fn initial_v2_imposition_has_a_real_judicial_root_and_unchanged_factual_decision() {
    let request = Fixture::single();
    let legacy = request.clone().capture();
    let fixture = FixtureV2::initial(request);
    let group = fixture.capture();
    let origin = MeasureOriginIds {
        decision_id: fixture.command.decision_id,
        operation_id: fixture.command.operation_id,
    };
    let result = &group.measures[0].result;
    assert_eq!(result.revision.get(), 1);
    assert_eq!(result.record_root, MeasureRecordRoot::Judicial(origin));
    assert_eq!(result.judicial_origin, origin);
    assert_eq!(result.action, MeasureCaptureAction::Impose);
    assert_eq!(result.previous, None);
    assert_eq!(result.effect_key, result.id);
    assert_eq!(group.decision, legacy.decision);
    assert_eq!(
        group.review.submission_digest,
        legacy.review.submission_digest
    );
    assert_ne!(group.review.review_digest, legacy.review.review_digest);
    assert!(measure_decision_review_v2_bytes(&group.review)
        .unwrap()
        .starts_with(b"MDPR2"));
    assert!(measure_capture_v2_bytes(&group.measures[0])
        .unwrap()
        .starts_with(b"MMCR2"));
    assert!(measure_decision_group_v2_bytes(&group)
        .unwrap()
        .starts_with(b"MDGR2"));
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
}

#[test]
fn no_measure_change_keeps_an_empty_v2_group_and_the_original_decision_encoding() {
    let request = Fixture::no_change();
    let legacy = request.clone().capture();
    let fixture = FixtureV2::initial(request);
    let group = fixture.capture();
    assert!(group.review.results.is_empty());
    assert!(group.measures.is_empty());
    assert!(group.substitutions.is_empty());
    assert_eq!(group.decision, legacy.decision);
    measure_group_origin_v2(&Hasher, &group, &fixture.history).unwrap();
}

#[test]
fn v2_confirm_accepts_a_real_v1_predecessor_without_repackaging_it() {
    let previous = Fixture::single().capture();
    let fixture = FixtureV2::from_v1(&previous, &crate::effect_support::empty_history());
    let group = fixture.capture();
    assert_eq!(
        group.measures[0].result.previous,
        Some(reference(&previous.measures[0]))
    );
    assert_eq!(
        group.measures[0].result.values,
        previous.measures[0].result.values
    );
    let OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(material)) =
        &group.review.material.predecessors[0]
    else {
        panic!("actual V1 material expected")
    };
    assert_eq!(material.as_ref(), &owned(&previous));
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
}

#[test]
fn modification_changes_declared_terms_but_retains_corrected_identity_and_origin() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let prior = &administrative.records[0].result;
    let mut fixture = FixtureV2::confirm(&administrative, &correction.history);
    let mut input = values_input(&prior.values);
    input.conditions = note("New judicially declared conditions");
    input.supervision = MeasureSupervision::Unknown {
        reason: note("Supervisor not declared"),
    };
    let values = MeasureValues::new(input);
    fixture.effects(vec![MeasureEffect::Modify {
        previous: record_reference(&administrative.records[0]),
        values: values.clone(),
    }]);
    fixture.material.result_sources[0].sources.supervisor = None;
    let group = fixture.capture();
    let result = &group.measures[0].result;
    assert_eq!(result.values, values);
    assert_eq!(result.values.subject(), prior.values.subject());
    assert_eq!(result.values.kind(), prior.values.kind());
    assert_eq!(result.record_root, prior.record_root);
    assert_eq!(result.judicial_origin, prior.judicial_origin);
    assert_eq!(result.action, MeasureCaptureAction::Modify);
    assert!(result.sources.supervisor.is_none());
    assert!(result.projection.supervisor.is_none());
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
}

#[test]
fn revoke_and_cease_retain_corrected_terms_without_inventing_an_end() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    for cease in [false, true] {
        let mut fixture = FixtureV2::confirm(&administrative, &correction.history);
        let previous = record_reference(&administrative.records[0]);
        fixture.effects(vec![if cease {
            MeasureEffect::Cease { previous }
        } else {
            MeasureEffect::Revoke { previous }
        }]);
        let group = fixture.capture();
        let result = &group.measures[0].result;
        assert_eq!(result.values, administrative.review.result.values);
        assert_eq!(result.sources, administrative.review.result.sources);
        assert_eq!(
            result.action,
            if cease {
                MeasureCaptureAction::Cease
            } else {
                MeasureCaptureAction::Revoke
            }
        );
        assert!(result.values.validity().end().is_none());
        measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
    }
}

#[test]
fn substitution_keeps_joint_links_and_assigns_fresh_successor_judicial_roots() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let prior = &administrative.records[0];
    let mut fixture = FixtureV2::confirm(&administrative, &correction.history);
    let previous = record_reference(prior);
    fixture.effects(vec![MeasureEffect::Substitute {
        predecessors: vec![previous],
        successors: vec![90, 80]
            .into_iter()
            .map(|value| MeasureProposal {
                id: id(value),
                values: prior.result.values.clone(),
            })
            .collect(),
    }]);
    fixture
        .material
        .result_sources
        .extend([90, 80].into_iter().map(|value| MeasureResultSources {
            id: id(value),
            sources: prior.result.sources.clone(),
        }));
    let group = fixture.capture();
    assert_eq!(group.measures.len(), 3);
    let outgoing = &group.measures[0];
    assert_eq!(outgoing.result.action, MeasureCaptureAction::SubstituteOut);
    assert_eq!(outgoing.result.values, prior.result.values);
    assert_eq!(outgoing.result.record_root, prior.result.record_root);
    let origin = MeasureOriginIds {
        decision_id: group.decision.decision_id,
        operation_id: group.decision.operation_id,
    };
    for successor in &group.measures[1..] {
        assert_eq!(successor.result.action, MeasureCaptureAction::SubstituteIn);
        assert_eq!(successor.result.revision.get(), 1);
        assert_eq!(successor.result.previous, None);
        assert_eq!(
            successor.result.record_root,
            MeasureRecordRoot::Judicial(origin)
        );
        assert_eq!(successor.result.judicial_origin, origin);
    }
    let relation = &group.substitutions[0];
    assert_eq!(relation.effect_key, id(70));
    assert_eq!(relation.predecessors.len(), 1);
    assert_eq!(relation.predecessors[0].previous, previous);
    assert_eq!(relation.predecessors[0].result, reference_v2(outgoing));
    assert_eq!(
        relation.successors,
        group.measures[1..]
            .iter()
            .map(reference_v2)
            .collect::<Vec<_>>()
    );
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
}
