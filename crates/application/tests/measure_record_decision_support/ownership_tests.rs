use super::*;
use crate::record_decision_support::{append_v2, empty_decision_history, reference_v2};
use uuid::Uuid;

#[path = "negative_support.rs"]
mod support;
use support::alternating;

#[test]
fn alternating_judicial_and_administrative_dependencies_require_every_exact_owner() {
    let original = alternating();
    original.clone().prepare().unwrap();
    for missing in 0..4 {
        let mut fixture = original.clone();
        match missing {
            0 => fixture.history.records.judicial.groups.clear(),
            1 => fixture.history.decisions.clear(),
            2 => {
                fixture.history.records.administrative.remove(0);
            }
            _ => {
                fixture.history.records.administrative.remove(1);
            }
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn v2_ancestors_require_all_origin_commitments_and_reject_duplicate_owners() {
    let original = alternating();
    for mutation in 0..9 {
        let mut fixture = original.clone();
        let origin = &mut fixture.history.decisions[0].origin;
        match mutation {
            0 => origin.case_id = domain::cases::CaseId::from_uuid(Uuid::from_u128(999)),
            1 => origin.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999)),
            2 => origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(999)),
            3 => origin.submission_digest = Sha256Digest::from_array([99; 32]),
            4 => origin.review_digest = Sha256Digest::from_array([99; 32]),
            5 => origin.decision_digest = Sha256Digest::from_array([99; 32]),
            6 => origin.group_digest = Sha256Digest::from_array([99; 32]),
            7 => fixture
                .history
                .decisions
                .push(fixture.history.decisions[0].clone()),
            _ => fixture
                .history
                .records
                .administrative
                .push(fixture.history.records.administrative[0].clone()),
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn v2_candidate_operation_and_decision_identities_cannot_recur_in_any_ancestor_family() {
    let original = alternating();
    let operations = [
        original.history.records.judicial.groups[0]
            .origin
            .operation_id
            .as_uuid(),
        original.history.decisions[0].origin.operation_id.as_uuid(),
        original.history.records.administrative[0]
            .origin
            .operation_id
            .as_uuid(),
        original.history.records.administrative[1]
            .origin
            .operation_id
            .as_uuid(),
    ];
    for operation in operations {
        let mut fixture = original.clone();
        fixture.command.operation_id = MeasureDecisionOperationId::from_uuid(operation);
        assert!(fixture.prepare().is_err());
    }
    for decision in [
        original.history.records.judicial.groups[0]
            .origin
            .decision_id,
        original.history.decisions[0].origin.decision_id,
    ] {
        let mut fixture = original.clone();
        fixture.command.decision_id = decision;
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn independent_v1_and_v2_groups_cannot_share_operation_or_decision_identity() {
    let (prior_fixture, prior) = corrected();
    for same_operation in [false, true] {
        let mut request = crate::measure_decision_fixtures::Fixture::single();
        if same_operation {
            request.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(999));
        } else {
            request.command.operation_id =
                MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999));
        }
        let MeasureEffect::Impose(mut proposal) =
            request.command.outcome.changes().unwrap()[0].clone()
        else {
            unreachable!()
        };
        proposal.id = id(90);
        request.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                MeasureEffect::Impose(proposal),
            ]))
            .unwrap();
        request.material.result_sources[0].id = id(90);
        let independent = FixtureV2::initial(request).capture();
        let mut evidence = empty_decision_history();
        evidence.records = append_administrative(&prior_fixture.history, &prior);
        evidence.decisions = append_v2(&empty_decision_history(), &independent).decisions;
        let selected = [
            record_reference(&prior.records[0]),
            reference_v2(&independent.measures[0]),
        ];
        assert!(resolve_measure_records_with_decision_history(
            &Hasher,
            prior_fixture.case_id,
            &selected,
            &evidence
        )
        .is_err());
    }
}

#[test]
fn v2_effect_cannot_replace_an_exact_correction_predecessor_with_its_old_judicial_capture() {
    let (first, prior) = corrected();
    let mut fixture = FixtureV2::confirm(&prior, &first.history);
    fixture.material.predecessors = vec![OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(
        Box::new(owned(&first.history.judicial.groups[0].capture)),
    ))];
    assert!(fixture.prepare().is_err());
}

#[test]
fn v2_administrative_cycle_shaped_dependencies_cannot_be_admitted_as_an_exact_closure() {
    let mut fixture = alternating();
    let latest = fixture.history.records.administrative[1].capture.clone();
    let selected = record_reference(&latest.records[0]);
    let group = &mut fixture.history.decisions[0].capture;
    group.review.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Confirm { previous: selected },
        ]))
        .unwrap();
    group.review.material.predecessors = vec![OwnedMeasureRecord::Administrative {
        owner: MeasureAdministrativeRef {
            operation_id: latest.review.command.operation_id,
            capture_digest: latest.capture_digest,
        },
        capture: Box::new(latest.records[0].clone()),
    }];
    group.review.results[0].previous = Some(selected);
    group.measures[0].result = group.review.results[0].clone();
    super::reconstruction::refresh(group);
    let entry = super::reconstruction::claimed_entry(group.clone());
    fixture.history.decisions[0] = entry;
    assert!(fixture.prepare().is_err());
}
