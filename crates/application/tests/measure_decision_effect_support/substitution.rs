use super::*;
use application::typed_participants::{CaseSubjectId, SubjectRevision};
use time::Duration;
use uuid::Uuid;

#[test]
fn many_to_many_substitution_retains_the_whole_relationship_without_pairing_members() {
    let previous = Fixture::multiple().capture();
    let fixture = substitution(&previous, &[100, 90, 110]);
    let evidence = fixture.evidence.clone();
    let group = fixture.capture();
    assert_eq!(group.measures.len(), 5);
    assert_eq!(group.substitutions.len(), 1);
    let relationship = &group.substitutions[0];
    assert_eq!(relationship.effect_key, id(70));
    assert_eq!(
        relationship.predecessors,
        previous
            .measures
            .iter()
            .zip(&group.measures[..2])
            .map(|(old, new)| MeasureSubstitutionPredecessor {
                previous: reference(old),
                result: reference(new),
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        relationship.successors,
        group.measures[2..]
            .iter()
            .map(reference)
            .collect::<Vec<_>>()
    );
    for (old, result) in previous.measures.iter().zip(&group.measures[..2]) {
        assert_eq!(result.result.action, MeasureCaptureAction::SubstituteOut);
        assert_eq!(result.result.previous, Some(reference(old)));
        assert_eq!(result.result.revision, MeasureRevision::new(2).unwrap());
        assert_eq!(result.result.origin, old.result.origin);
        assert_eq!(result.result.values, old.result.values);
        assert_eq!(result.result.sources, old.result.sources);
        assert!(result.result.values.validity().end().is_none());
    }
    for result in &group.measures[2..] {
        assert_eq!(result.result.action, MeasureCaptureAction::SubstituteIn);
        assert_eq!(result.result.previous, None);
        assert_eq!(result.result.revision, MeasureRevision::initial());
        assert_eq!(
            result.result.origin,
            MeasureOriginIds {
                operation_id: group.decision.operation_id,
                decision_id: group.decision.decision_id,
            }
        );
    }
    assert!(group
        .measures
        .iter()
        .all(|member| member.result.effect_key == id(70)));
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}

#[test]
fn substitute_successor_can_select_a_new_exact_revision_of_the_same_subject() {
    let previous = Fixture::single().capture();
    let mut fixture = substitution(&previous, &[90]);
    let successor = &mut fixture.request.material.result_sources[1];
    successor.sources.subject.revision = SubjectRevision::new(8).unwrap();
    successor.sources.subject.changed_at += Duration::seconds(1);
    successor.sources.subject.changed_by.email = "newer-subject@example.test".into();
    let mut input = values_input(&previous.measures[0].result.values);
    input.subject = crate::measure_source_support::subject_ref(&successor.sources.subject);
    let values = MeasureValues::new(input);
    fixture.effects(vec![MeasureEffect::Substitute {
        predecessors: vec![reference(&previous.measures[0])],
        successors: vec![MeasureProposal {
            id: id(90),
            values: values.clone(),
        }],
    }]);
    let evidence = fixture.evidence.clone();
    let group = fixture.capture();
    assert_eq!(
        group.measures[0].result.values.subject().revision,
        SubjectRevision::new(7).unwrap()
    );
    assert_eq!(group.measures[1].result.values, values);
    assert_eq!(
        group.measures[1].result.values.subject().id,
        group.measures[0].result.values.subject().id
    );
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}

#[test]
fn substitution_rejects_a_different_subject_even_when_its_displayed_name_matches() {
    let previous = Fixture::single().capture();
    let mut fixture = substitution(&previous, &[90]);
    let successor = &mut fixture.request.material.result_sources[1];
    successor.sources.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(61));
    let mut input = values_input(&previous.measures[0].result.values);
    input.subject = crate::measure_source_support::subject_ref(&successor.sources.subject);
    let values = MeasureValues::new(input);
    resolve_measure_sources(
        &Hasher,
        fixture.request.case_id,
        &values,
        &successor.sources,
    )
    .unwrap();
    fixture.effects(vec![MeasureEffect::Substitute {
        predecessors: vec![reference(&previous.measures[0])],
        successors: vec![MeasureProposal { id: id(90), values }],
    }]);
    assert!(fixture.prepare().is_err());
}

#[test]
fn substitution_rejects_multiple_old_subjects_even_when_all_sources_are_individually_valid() {
    let mut initial = Fixture::multiple();
    let source = &mut initial.material.result_sources[1].sources;
    source.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(61));
    let values = MeasureValues::new(crate::measure_source_support::input(source));
    initial.replace_values(id(80), values);
    let previous = initial.capture();
    assert!(substitution(&previous, &[90]).prepare().is_err());
}

#[test]
fn replacement_outputs_cannot_rewrite_the_retained_source_of_an_old_measure() {
    let previous = Fixture::multiple().capture();
    let mut fixture = substitution(&previous, &[90]);
    fixture.request.material.result_sources[1]
        .sources
        .subject
        .changed_by
        .email = "rewritten@example.test".into();
    assert!(fixture.prepare().is_err());
}

#[test]
fn substitution_normalizes_all_thirty_two_old_and_new_members_without_truncation() {
    let mut initial = Fixture::single();
    let sources = initial.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&sources));
    for value in 71..86 {
        initial.add_imposition(value, values.clone(), sources.clone());
    }
    let previous = initial.capture();
    let successors = (90..106).rev().collect::<Vec<_>>();
    let fixture = substitution(&previous, &successors);
    let expected = fixture.clone().capture();
    let mut reversed = fixture;
    reversed.request.material.predecessors.reverse();
    reversed.request.material.result_sources.reverse();
    let group = reversed.capture();
    assert_eq!(group, expected);
    assert_eq!(group.measures.len(), 32);
    assert_eq!(group.substitutions[0].predecessors.len(), 16);
    assert_eq!(group.substitutions[0].successors.len(), 16);
    assert_eq!(
        group
            .measures
            .iter()
            .map(|member| member.result.id)
            .collect::<Vec<_>>(),
        (70..86).chain(90..106).map(id).collect::<Vec<_>>()
    );
}

#[test]
fn terminal_revoked_ceased_and_substituted_out_identities_reject_all_later_effects() {
    let initial = Fixture::single().capture();
    for terminal_action in 0..3 {
        let mut terminal_fixture = LaterFixture::confirm(&initial);
        let previous = reference(&initial.measures[0]);
        if terminal_action == 2 {
            terminal_fixture = substitution(&initial, &[80]);
        } else {
            terminal_fixture.effects(vec![if terminal_action == 0 {
                MeasureEffect::Revoke { previous }
            } else {
                MeasureEffect::Cease { previous }
            }]);
        }
        let ancestors = terminal_fixture.evidence.clone();
        let terminal = terminal_fixture.capture();
        let old = &terminal.measures[0];
        for action in 0..5 {
            let mut fixture = LaterFixture::next(&terminal, &ancestors, 2);
            let previous = reference(old);
            let effect = match action {
                0 => MeasureEffect::Confirm { previous },
                1 => MeasureEffect::Modify {
                    previous,
                    values: old.result.values.clone(),
                },
                2 => MeasureEffect::Revoke { previous },
                3 => MeasureEffect::Cease { previous },
                _ => {
                    fixture
                        .request
                        .material
                        .result_sources
                        .push(MeasureResultSources {
                            id: id(90),
                            sources: old.result.sources.clone(),
                        });
                    MeasureEffect::Substitute {
                        predecessors: vec![previous],
                        successors: vec![MeasureProposal {
                            id: id(90),
                            values: old.result.values.clone(),
                        }],
                    }
                }
            };
            fixture.effects(vec![effect]);
            assert!(
                fixture.prepare().is_err(),
                "terminal action {terminal_action}, later action {action}"
            );
        }
    }
}

#[test]
fn shared_substitution_table_must_match_every_old_and_new_result_after_rehashing() {
    let previous = Fixture::multiple().capture();
    let fixture = substitution(&previous, &[90, 100]);
    let evidence = fixture.evidence.clone();
    let baseline = fixture.capture();
    for mutation in 0..6 {
        let mut group = baseline.clone();
        let relationship = &mut group.substitutions[0];
        match mutation {
            0 => {
                relationship.predecessors.pop();
            }
            1 => {
                relationship.successors.pop();
            }
            2 => relationship.predecessors.swap(0, 1),
            3 => relationship.successors.swap(0, 1),
            4 => relationship.effect_key = id(90),
            _ => relationship.predecessors[0].result = relationship.successors[0],
        }
        refresh_group_digest(&mut group);
        assert!(measure_decision_group_with_history_matches(&Hasher, &group, &evidence).is_err());
    }
}
