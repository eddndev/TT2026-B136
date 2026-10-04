use crate::{decision_support::*, measure_history_support::*};
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use uuid::Uuid;

#[test]
fn group_origin_retains_all_exact_commitments_and_creation_identities() {
    let group = Fixture::multiple().capture();
    let origin = measure_group_origin(&Hasher, &group, &empty()).unwrap();
    assert_eq!(origin, claimed_entry(group.clone()).origin);
    let evidence = history(vec![MeasureGroupEvidence {
        origin,
        capture: group.clone(),
    }]);
    let selections = group.measures.iter().map(reference).collect::<Vec<_>>();
    let actual =
        resolve_measure_targets(&Hasher, group.review.case_id, &selections, &evidence).unwrap();
    assert_eq!(actual.targets(), &[member(&group, 70), member(&group, 80)]);
}

#[test]
fn every_origin_field_must_match_the_validated_group() {
    let group = Fixture::single().capture();
    for mutation in 0..7 {
        let mut source = entry(&group, &empty());
        let origin = &mut source.origin;
        let digest = Sha256Digest::from_array([99; 32]);
        match mutation {
            0 => origin.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => origin.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(99)),
            2 => origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(99)),
            3 => origin.submission_digest = digest,
            4 => origin.review_digest = digest,
            5 => origin.decision_digest = digest,
            _ => origin.group_digest = digest,
        }
        rejects(&group, &history(vec![source]));
    }
}

#[test]
fn selection_requires_exact_measure_identity_revision_digest_and_case() {
    let group = Fixture::single().capture();
    let evidence = history(vec![entry(&group, &empty())]);
    let selected = reference(&group.measures[0]);
    for value in [
        PrecautionaryMeasureRef::new(id(99), selected.revision(), selected.digest()),
        PrecautionaryMeasureRef::new(
            selected.id(),
            MeasureRevision::new(2).unwrap(),
            selected.digest(),
        ),
        PrecautionaryMeasureRef::new(
            selected.id(),
            selected.revision(),
            Sha256Digest::from_array([99; 32]),
        ),
    ] {
        assert!(
            resolve_measure_targets(&Hasher, group.review.case_id, &[value], &evidence).is_err()
        );
    }
    assert!(resolve_measure_targets(
        &Hasher,
        CaseId::from_uuid(Uuid::from_u128(99)),
        &[selected],
        &evidence,
    )
    .is_err());
}

#[test]
fn complete_owner_validation_rejects_a_rehashed_unselected_sibling() {
    for mutation in 0..3 {
        let mut group = Fixture::multiple().capture();
        match mutation {
            0 => {
                group.measures[1].result.origin.decision_id =
                    MeasureDecisionId::from_uuid(Uuid::from_u128(99))
            }
            1 => {
                group.measures[1].result.projection.subject.display_name =
                    "Invented sibling label".into()
            }
            _ => {
                group.measures[1].result.sources.subject.changed_by.email =
                    "invented@example.test".into()
            }
        }
        refresh_digests(&mut group);
        assert!(measure_group_origin(&Hasher, &group, &empty()).is_err());
        rejects(&group, &history(vec![claimed_entry(group.clone())]));
    }
}

#[test]
fn missing_or_invented_owner_members_reject_even_when_all_digests_are_refreshed() {
    for mutation in 0..3 {
        let mut group = Fixture::multiple().capture();
        match mutation {
            0 => {
                group.measures.pop();
            }
            1 => {
                let mut invented = group.measures[1].clone();
                invented.result.id = id(90);
                invented.result.effect_key = id(90);
                group.measures.push(invented);
            }
            _ => {
                group.measures[1].result.id = id(90);
                group.measures[1].result.effect_key = id(90);
            }
        }
        refresh_digests(&mut group);
        assert!(measure_group_origin(&Hasher, &group, &empty()).is_err());
        rejects(&group, &history(vec![claimed_entry(group.clone())]));
    }
}

#[test]
fn an_empty_selection_requires_empty_evidence() {
    let group = Fixture::single().capture();
    let empty_evidence = empty();
    let actual =
        resolve_measure_targets(&Hasher, group.review.case_id, &[], &empty_evidence).unwrap();
    assert!(actual.targets().is_empty());
    let evidence = history(vec![entry(&group, &empty())]);
    assert!(resolve_measure_targets(&Hasher, group.review.case_id, &[], &evidence).is_err());
}
