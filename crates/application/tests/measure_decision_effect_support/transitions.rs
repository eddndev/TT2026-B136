use super::*;
use application::typed_participants::{
    subject_digest, CaseSubjectId, ParticipantText, RepresentedName, SubjectRevision, SubjectValues,
};
use domain::hearings::{HearingNote, HearingParticipantRef};
use domain::participants::DirectoryStatus;
use domain::procedural_time::DeclaredProceduralTime;
use time::Duration;
use uuid::Uuid;

#[test]
fn modify_preserves_exact_subject_kind_and_origin_while_resolving_new_declared_terms() {
    let previous = Fixture::single().capture();
    let prior = &previous.measures[0];
    let mut fixture = LaterFixture::confirm(&previous);
    let sources = &mut fixture.request.material.result_sources[0].sources;
    sources.supervisor = Some(crate::participant_support::manual(
        8,
        1,
        DirectoryStatus::Archived,
    ));
    let mut input = values_input(&prior.result.values);
    input.conditions = HearingNote::new("New expressly declared conditions").unwrap();
    input.validity = MeasureValidity::new(
        prior.result.values.validity().start().clone(),
        HearingNote::new("New expressly declared validity").unwrap(),
        Some(
            MeasureTime::new(
                DeclaredProceduralTime::unknown(),
                Some(HearingNote::new("End not stated in the decision").unwrap()),
            )
            .unwrap(),
        ),
    )
    .unwrap();
    let supervisor = sources.supervisor.as_ref().unwrap();
    input.supervision = MeasureSupervision::Known {
        participant: HearingParticipantRef::new(supervisor.id(), supervisor.revision_number()),
        statement: HearingNote::new("New supervisor named in the decision").unwrap(),
    };
    let values = MeasureValues::new(input);
    fixture.effects(vec![MeasureEffect::Modify {
        previous: reference(prior),
        values: values.clone(),
    }]);
    let evidence = fixture.evidence.clone();
    let group = fixture.capture();
    let result = &group.measures[0].result;
    assert_eq!(result.action, MeasureCaptureAction::Modify);
    assert_eq!(result.revision, MeasureRevision::new(2).unwrap());
    assert_eq!(result.previous, Some(reference(prior)));
    assert_eq!(result.origin, prior.result.origin);
    assert_eq!(result.values, values);
    assert_eq!(result.values.subject(), prior.result.values.subject());
    assert_eq!(result.values.kind(), prior.result.values.kind());
    assert_eq!(
        result
            .projection
            .supervisor
            .as_ref()
            .unwrap()
            .overview
            .display_name,
        "Historical manual name"
    );
    assert_eq!(
        result
            .sources
            .supervisor
            .as_ref()
            .unwrap()
            .revision_number()
            .get(),
        1
    );
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}

#[test]
fn modify_rejects_new_subject_id_revision_digest_or_measure_kind_even_with_resolved_material() {
    let previous = Fixture::single().capture();
    for mutation in 0..4 {
        let mut fixture = LaterFixture::confirm(&previous);
        let sources = &mut fixture.request.material.result_sources[0].sources;
        match mutation {
            0 => sources.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(61)),
            1 => sources.subject.revision = SubjectRevision::new(8).unwrap(),
            2 => {
                let SubjectValues::NaturalPerson { name, .. } = &mut sources.subject.values else {
                    panic!("natural person fixture expected")
                };
                *name =
                    RepresentedName::Known(ParticipantText::new("Changed subject values").unwrap());
                sources.subject.values_digest = subject_digest(&Hasher, &sources.subject.values);
            }
            _ => {}
        }
        let mut input = values_input(&previous.measures[0].result.values);
        input.subject = crate::measure_source_support::subject_ref(&sources.subject);
        if mutation == 3 {
            input.kind = MeasureKind::FinancialGuarantee;
        }
        let values = MeasureValues::new(input);
        resolve_measure_sources(&Hasher, fixture.request.case_id, &values, sources).unwrap();
        fixture.effects(vec![MeasureEffect::Modify {
            previous: reference(&previous.measures[0]),
            values,
        }]);
        assert!(
            fixture.prepare().is_err(),
            "immutable field mutation {mutation}"
        );
    }
}

#[test]
fn revoke_and_cease_retain_complete_terms_and_sources_without_inserting_an_end() {
    let previous = Fixture::single().capture();
    let prior = &previous.measures[0];
    for cease in [false, true] {
        let mut fixture = LaterFixture::confirm(&previous);
        fixture.effects(vec![if cease {
            MeasureEffect::Cease {
                previous: reference(prior),
            }
        } else {
            MeasureEffect::Revoke {
                previous: reference(prior),
            }
        }]);
        let evidence = fixture.evidence.clone();
        let group = fixture.capture();
        let result = &group.measures[0].result;
        assert_eq!(
            result.action,
            if cease {
                MeasureCaptureAction::Cease
            } else {
                MeasureCaptureAction::Revoke
            }
        );
        assert_eq!(result.values, prior.result.values);
        assert_eq!(result.sources, prior.result.sources);
        assert_eq!(result.projection, prior.result.projection);
        assert_eq!(result.origin, prior.result.origin);
        assert_eq!(result.previous, Some(reference(prior)));
        assert_eq!(result.revision, MeasureRevision::new(2).unwrap());
        assert!(result.values.validity().end().is_none());
        measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
    }
}

#[test]
fn retained_value_effects_reject_rewritten_full_source_provenance() {
    let previous = Fixture::single().capture();
    let selected = reference(&previous.measures[0]);
    for effect in [
        MeasureEffect::Confirm { previous: selected },
        MeasureEffect::Revoke { previous: selected },
        MeasureEffect::Cease { previous: selected },
    ] {
        for location in 0..3 {
            let mut fixture = LaterFixture::confirm(&previous);
            fixture.effects(vec![effect.clone()]);
            let sources = &mut fixture.request.material.result_sources[0].sources;
            match location {
                0 => sources.subject.changed_by.email = "rewritten@example.test".into(),
                1 => {
                    crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
                        .changed_at += Duration::seconds(1)
                }
                _ => {
                    sources
                        .supervisor
                        .as_mut()
                        .unwrap()
                        .bound_subject
                        .as_mut()
                        .unwrap()
                        .changed_by
                        .email = "rewritten@example.test".into()
                }
            }
            resolve_measure_sources(
                &Hasher,
                fixture.request.case_id,
                &previous.measures[0].result.values,
                sources,
            )
            .unwrap();
            assert!(fixture.prepare().is_err());
        }
    }
}

#[test]
fn confirm_after_modify_retains_the_modified_terms_and_original_creation_ids() {
    let first = Fixture::single().capture();
    let confirmed_fixture = LaterFixture::confirm(&first);
    let first_evidence = confirmed_fixture.evidence.clone();
    let second = confirmed_fixture.capture();
    let mut modified_fixture = LaterFixture::next(&second, &first_evidence, 2);
    let mut input = values_input(&second.measures[0].result.values);
    input.conditions = HearingNote::new("Modified declared conditions").unwrap();
    let changed = MeasureValues::new(input);
    modified_fixture.effects(vec![MeasureEffect::Modify {
        previous: reference(&second.measures[0]),
        values: changed.clone(),
    }]);
    let second_evidence = modified_fixture.evidence.clone();
    let third = modified_fixture.capture();
    let fourth_fixture = LaterFixture::next(&third, &second_evidence, 3);
    let evidence = fourth_fixture.evidence.clone();
    let fourth = fourth_fixture.capture();
    assert_eq!(
        fourth.measures[0].result.revision,
        MeasureRevision::new(4).unwrap()
    );
    assert_eq!(fourth.measures[0].result.values, changed);
    assert_eq!(
        fourth.measures[0].result.origin,
        first.measures[0].result.origin
    );
    assert_eq!(
        fourth.measures[0].result.previous,
        Some(reference(&third.measures[0]))
    );
    measure_decision_group_with_history_matches(&Hasher, &fourth, &evidence).unwrap();
}

#[test]
fn a_group_can_mix_retained_modified_and_new_measure_identities() {
    let previous = Fixture::multiple().capture();
    let mut fixture = LaterFixture::confirm(&previous);
    let old = &previous.measures;
    let mut modified = values_input(&old[1].result.values);
    modified.conditions = HearingNote::new("Updated second measure").unwrap();
    fixture.effects(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: id(90),
            values: old[0].result.values.clone(),
        }),
        MeasureEffect::Modify {
            previous: reference(&old[1]),
            values: MeasureValues::new(modified),
        },
        MeasureEffect::Confirm {
            previous: reference(&old[0]),
        },
    ]);
    fixture.request.material.predecessors = old
        .iter()
        .rev()
        .map(|member| owned_member(&previous, member))
        .collect();
    fixture.request.material.result_sources = vec![
        MeasureResultSources {
            id: id(90),
            sources: old[0].result.sources.clone(),
        },
        MeasureResultSources {
            id: old[1].result.id,
            sources: old[1].result.sources.clone(),
        },
        MeasureResultSources {
            id: old[0].result.id,
            sources: old[0].result.sources.clone(),
        },
    ];
    let evidence = fixture.evidence.clone();
    let group = fixture.capture();
    assert_eq!(
        group
            .measures
            .iter()
            .map(|m| m.result.id)
            .collect::<Vec<_>>(),
        vec![id(70), id(80), id(90)]
    );
    assert_eq!(
        group.measures[0].result.action,
        MeasureCaptureAction::Confirm
    );
    assert_eq!(
        group.measures[1].result.action,
        MeasureCaptureAction::Modify
    );
    assert_eq!(
        group.measures[2].result.action,
        MeasureCaptureAction::Impose
    );
    assert_eq!(group.measures[0].result.origin, old[0].result.origin);
    assert_eq!(group.measures[1].result.origin, old[1].result.origin);
    assert_eq!(
        group.measures[2].result.origin,
        MeasureOriginIds {
            decision_id: group.decision.decision_id,
            operation_id: group.decision.operation_id
        }
    );
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}
