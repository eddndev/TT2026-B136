pub use crate::measure_decision_fixtures::*;
use application::ApplicationError;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn empty_history() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn append_history(
    ancestors: &MeasureHistoryEvidence,
    group: &MeasureDecisionGroupCapture,
) -> MeasureHistoryEvidence {
    let origin = measure_group_origin(&Hasher, group, ancestors).unwrap();
    let mut evidence = ancestors.clone();
    evidence.groups.push(MeasureGroupEvidence {
        origin,
        capture: group.clone(),
    });
    evidence
}

pub fn owned_member(
    group: &MeasureDecisionGroupCapture,
    measure: &MeasureCapture,
) -> OwnedMeasureMaterial {
    OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: group.decision.operation_id,
            decision_id: group.decision.decision_id,
            group_digest: group.capture_digest,
        },
        capture: measure.clone(),
    }
}

pub fn values_input(values: &MeasureValues) -> MeasureValuesInput {
    MeasureValuesInput {
        subject: values.subject(),
        kind: values.kind(),
        conditions: values.conditions().clone(),
        validity: values.validity().clone(),
        supervision: values.supervision().clone(),
    }
}

pub fn substitution(
    previous: &MeasureDecisionGroupCapture,
    successor_ids: &[u128],
) -> LaterFixture {
    let mut fixture = LaterFixture::confirm(previous);
    let first = &previous.measures[0].result;
    fixture.effects(vec![MeasureEffect::Substitute {
        predecessors: previous.measures.iter().map(reference).collect(),
        successors: successor_ids
            .iter()
            .map(|value| MeasureProposal {
                id: id(*value),
                values: first.values.clone(),
            })
            .collect(),
    }]);
    fixture.request.material.predecessors = previous
        .measures
        .iter()
        .map(|member| owned_member(previous, member))
        .collect();
    fixture.request.material.result_sources = previous
        .measures
        .iter()
        .map(|member| MeasureResultSources {
            id: member.result.id,
            sources: member.result.sources.clone(),
        })
        .collect();
    fixture
        .request
        .material
        .result_sources
        .extend(successor_ids.iter().map(|value| MeasureResultSources {
            id: id(*value),
            sources: first.sources.clone(),
        }));
    fixture
}

#[derive(Clone)]
pub struct LaterFixture {
    pub request: Fixture,
    pub evidence: MeasureHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}

impl LaterFixture {
    pub fn confirm(previous: &MeasureDecisionGroupCapture) -> Self {
        Self::next(previous, &empty_history(), 1)
    }

    pub fn next(
        previous: &MeasureDecisionGroupCapture,
        ancestors: &MeasureHistoryEvidence,
        sequence: u128,
    ) -> Self {
        let prior = &previous.measures[0];
        let mut request = Fixture::single();
        request.command.operation_id =
            MeasureDecisionOperationId::from_uuid(Uuid::from_u128(100 + sequence));
        request.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(110 + sequence));
        request.material.context = previous.review.material.context.clone();
        request.command.context = expectation(&request.material.context);
        request.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                MeasureEffect::Confirm {
                    previous: reference(prior),
                },
            ]))
            .unwrap();
        request.material.predecessors = vec![owned(previous)];
        request.material.result_sources = vec![MeasureResultSources {
            id: prior.result.id,
            sources: prior.result.sources.clone(),
        }];
        Self {
            request,
            evidence: append_history(ancestors, previous),
            recorded_at: previous.recorded_at + Duration::seconds(1),
        }
    }

    pub fn effects(&mut self, effects: Vec<MeasureEffect>) {
        self.request.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    }

    pub fn prepare(self) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
        prepare_measure_decision_with_history(
            &Hasher,
            &self.request.actor,
            self.request.case_id,
            self.request.command,
            self.request.material,
            &self.evidence,
        )
    }

    pub fn capture(self) -> MeasureDecisionGroupCapture {
        let recorded_at = self.recorded_at;
        self.prepare()
            .unwrap()
            .into_group_capture(&Hasher, recorded_at)
            .unwrap()
    }
}
