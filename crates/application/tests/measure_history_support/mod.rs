use crate::decision_support::*;
use domain::clock::OffsetDateTime;
use time::Duration;
use uuid::Uuid;

pub fn empty() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn history(groups: Vec<MeasureGroupEvidence>) -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups }
}

pub fn entry(
    group: &MeasureDecisionGroupCapture,
    ancestors: &MeasureHistoryEvidence,
) -> MeasureGroupEvidence {
    MeasureGroupEvidence {
        origin: measure_group_origin(&Hasher, group, ancestors).unwrap(),
        capture: group.clone(),
    }
}

pub fn claimed_entry(group: MeasureDecisionGroupCapture) -> MeasureGroupEvidence {
    MeasureGroupEvidence {
        origin: MeasureGroupOrigin {
            case_id: group.review.case_id,
            operation_id: group.review.command.operation_id,
            decision_id: group.review.command.decision_id,
            submission_digest: group.review.submission_digest,
            review_digest: group.review.review_digest,
            decision_digest: group.decision.capture_digest,
            group_digest: group.capture_digest,
        },
        capture: group,
    }
}

pub fn root_fixture(serial: u128, measure: u128) -> Fixture {
    let mut fixture = Fixture::single();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(2000 + serial));
    let source = &mut fixture.material.result_sources[0];
    source.id = id(measure);
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(MeasureProposal {
                id: id(measure),
                values: MeasureValues::new(crate::measure_source_support::input(&source.sources)),
            }),
        ]))
        .unwrap();
    fixture
}

pub fn member(group: &MeasureDecisionGroupCapture, measure: u128) -> OwnedMeasureMaterial {
    OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: group.decision.operation_id,
            decision_id: group.decision.decision_id,
            group_digest: group.capture_digest,
        },
        capture: group
            .measures
            .iter()
            .find(|item| item.result.id == id(measure))
            .unwrap()
            .clone(),
    }
}

pub fn confirmation(
    serial: u128,
    targets: &[OwnedMeasureMaterial],
    evidence: &MeasureHistoryEvidence,
    recorded_at: OffsetDateTime,
) -> MeasureDecisionGroupCapture {
    let mut fixture = root_fixture(serial, 999);
    fixture.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        targets
            .iter()
            .map(|item| MeasureEffect::Confirm {
                previous: reference(&item.capture),
            })
            .collect(),
    ))
    .unwrap();
    fixture.material.predecessors = targets.to_vec();
    fixture.material.result_sources = targets
        .iter()
        .map(|item| MeasureResultSources {
            id: item.capture.result.id,
            sources: item.capture.result.sources.clone(),
        })
        .collect();
    prepare_measure_decision_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.material,
        evidence,
    )
    .unwrap()
    .into_group_capture(&Hasher, recorded_at)
    .unwrap()
}

pub fn append(evidence: &mut MeasureHistoryEvidence, group: &MeasureDecisionGroupCapture) {
    let next = entry(group, evidence);
    evidence.groups.push(next);
}

pub fn three_revisions() -> (MeasureDecisionGroupCapture, MeasureHistoryEvidence) {
    let root = root_fixture(1, 70).capture();
    let mut evidence = history(vec![entry(&root, &empty())]);
    let second = confirmation(
        2,
        &[member(&root, 70)],
        &evidence,
        at() + Duration::seconds(1),
    );
    append(&mut evidence, &second);
    let third = confirmation(
        3,
        &[member(&second, 70)],
        &evidence,
        at() + Duration::seconds(2),
    );
    append(&mut evidence, &third);
    (third, evidence)
}

pub fn rejects(group: &MeasureDecisionGroupCapture, evidence: &MeasureHistoryEvidence) {
    let selected = reference(&group.measures[0]);
    assert!(resolve_measure_targets(&Hasher, group.review.case_id, &[selected], evidence).is_err());
}
