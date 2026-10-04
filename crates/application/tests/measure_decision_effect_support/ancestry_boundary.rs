use super::*;
use domain::crypto::{DocumentHasher, Sha256Digest};
use uuid::Uuid;

#[test]
fn later_effects_require_exact_owning_history_and_remain_rejected_by_empty_evidence_wrappers() {
    let previous = Fixture::single().capture();
    let fixture = LaterFixture::confirm(&previous);
    assert!(fixture.request.clone().prepare().is_err());
    let mut missing = fixture.clone();
    missing.evidence = empty_history();
    assert!(missing.prepare().is_err());
    let group = fixture.capture();
    assert!(measure_decision_group_matches(&Hasher, &group).is_err());
    assert!(
        measure_decision_group_with_history_matches(&Hasher, &group, &empty_history()).is_err()
    );
}

#[test]
fn exact_predecessor_inventory_rejects_missing_extra_duplicate_and_wrong_ownership() {
    let previous = Fixture::multiple().capture();
    for mutation in 0..8 {
        let mut fixture = LaterFixture::confirm(&previous);
        let predecessors = &mut fixture.request.material.predecessors;
        match mutation {
            0 => predecessors.clear(),
            1 => predecessors.push(owned_member(&previous, &previous.measures[1])),
            2 => predecessors.push(predecessors[0].clone()),
            3 => predecessors[0] = owned_member(&previous, &previous.measures[1]),
            4 => {
                predecessors[0].owner.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
            }
            5 => {
                predecessors[0].owner.decision_id =
                    MeasureDecisionId::from_uuid(Uuid::from_u128(999))
            }
            6 => predecessors[0].owner.group_digest = Sha256Digest::from_array([99; 32]),
            _ => {
                predecessors[0]
                    .capture
                    .result
                    .sources
                    .subject
                    .changed_by
                    .email = "changed@example.test".into()
            }
        }
        assert!(
            fixture.prepare().is_err(),
            "predecessor mutation {mutation}"
        );
    }
}

#[test]
fn every_affected_later_measure_requires_one_exact_result_source() {
    let previous = Fixture::single().capture();
    for mutation in 0..4 {
        let mut fixture = LaterFixture::confirm(&previous);
        let sources = &mut fixture.request.material.result_sources;
        match mutation {
            0 => sources.clear(),
            1 => sources.push(sources[0].clone()),
            2 => sources[0].id = id(80),
            _ => sources.extend(vec![sources[0].clone(); 33]),
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn new_imposition_and_substitution_ids_cannot_reuse_an_unselected_sibling_in_the_closure() {
    let previous = Fixture::multiple().capture();
    let old = &previous.measures[0];
    for substitute in [false, true] {
        let mut fixture = LaterFixture::confirm(&previous);
        let proposal = MeasureProposal {
            id: id(80),
            values: old.result.values.clone(),
        };
        fixture.effects(if substitute {
            vec![MeasureEffect::Substitute {
                predecessors: vec![reference(old)],
                successors: vec![proposal],
            }]
        } else {
            vec![
                MeasureEffect::Confirm {
                    previous: reference(old),
                },
                MeasureEffect::Impose(proposal),
            ]
        });
        fixture
            .request
            .material
            .result_sources
            .push(MeasureResultSources {
                id: id(80),
                sources: old.result.sources.clone(),
            });
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn candidate_operation_and_decision_ids_cannot_repeat_any_ancestor_identity() {
    let previous = Fixture::single().capture();
    for operation in [false, true] {
        let mut fixture = LaterFixture::confirm(&previous);
        if operation {
            fixture.request.command.operation_id = previous.decision.operation_id;
        } else {
            fixture.request.command.decision_id = previous.decision.decision_id;
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn rehashed_naked_predecessor_cannot_forge_a_maximum_revision_or_creation_origin() {
    let previous = Fixture::single().capture();
    for mutation in 0..3 {
        let mut fixture = LaterFixture::confirm(&previous);
        let capture = &mut fixture.request.material.predecessors[0].capture;
        match mutation {
            0 => capture.result.revision = MeasureRevision::new(u32::MAX).unwrap(),
            1 => {
                capture.result.origin.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
            }
            _ => {
                capture.result.origin.decision_id =
                    MeasureDecisionId::from_uuid(Uuid::from_u128(999))
            }
        }
        capture.capture_digest = Hasher.hash_bytes(&measure_capture_bytes(capture).unwrap());
        let selected = reference(capture);
        fixture.effects(vec![MeasureEffect::Confirm { previous: selected }]);
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn a_supplied_valid_prefix_does_not_claim_that_a_later_group_is_absent() {
    let first = Fixture::single().capture();
    let later = LaterFixture::confirm(&first).capture();
    let alternative = LaterFixture::next(&first, &empty_history(), 2);
    let evidence = alternative.evidence.clone();
    let result = alternative.capture();
    assert_ne!(result.decision.operation_id, later.decision.operation_id);
    assert_eq!(
        result.measures[0].result.previous,
        Some(reference(&first.measures[0]))
    );
    measure_decision_group_with_history_matches(&Hasher, &result, &evidence).unwrap();
}

#[test]
fn thirty_two_existing_targets_normalize_without_losing_an_exact_predecessor() {
    let mut initial = Fixture::single();
    let source = initial.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&source));
    for value in 71..102 {
        initial.add_imposition(value, values.clone(), source.clone());
    }
    let previous = initial.capture();
    let mut fixture = LaterFixture::confirm(&previous);
    fixture.effects(
        previous
            .measures
            .iter()
            .rev()
            .map(|member| MeasureEffect::Confirm {
                previous: reference(member),
            })
            .collect(),
    );
    fixture.request.material.predecessors = previous
        .measures
        .iter()
        .rev()
        .map(|member| owned_member(&previous, member))
        .collect();
    fixture.request.material.result_sources = previous
        .measures
        .iter()
        .rev()
        .map(|member| MeasureResultSources {
            id: member.result.id,
            sources: member.result.sources.clone(),
        })
        .collect();
    let evidence = fixture.evidence.clone();
    let group = fixture.capture();
    assert_eq!(group.measures.len(), 32);
    assert_eq!(group.review.material.predecessors.len(), 32);
    for (old, new) in previous.measures.iter().zip(&group.measures) {
        assert_eq!(new.result.id, old.result.id);
        assert_eq!(new.result.previous, Some(reference(old)));
        assert_eq!(new.result.revision, MeasureRevision::new(2).unwrap());
        assert_eq!(new.result.origin, old.result.origin);
    }
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}
