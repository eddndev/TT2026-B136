pub use crate::record_support::*;

use application::identity::Principal;
use application::ApplicationError;
use domain::cases::CaseId;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn empty_decision_history() -> MeasureDecisionRecordHistoryEvidence {
    MeasureDecisionRecordHistoryEvidence {
        records: empty(),
        decisions: vec![],
    }
}

pub fn reference_v2(capture: &MeasureCaptureV2) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

pub fn owned_v2(group: &MeasureDecisionGroupCaptureV2, index: usize) -> OwnedMeasureRecord {
    OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(Box::new(OwnedMeasureMaterialV2 {
        owner: MeasureGroupRef {
            operation_id: group.decision.operation_id,
            decision_id: group.decision.decision_id,
            group_digest: group.capture_digest,
        },
        capture: group.measures[index].clone(),
    })))
}

pub fn append_v2(
    history: &MeasureDecisionRecordHistoryEvidence,
    group: &MeasureDecisionGroupCaptureV2,
) -> MeasureDecisionRecordHistoryEvidence {
    let origin = measure_group_origin_v2(&Hasher, group, history).unwrap();
    let mut result = history.clone();
    result.decisions.push(MeasureGroupEvidenceV2 {
        origin,
        capture: group.clone(),
    });
    result
}

#[derive(Clone)]
pub struct FixtureV2 {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureDecisionCommand,
    pub material: MeasureDecisionMaterialV2,
    pub history: MeasureDecisionRecordHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}

impl FixtureV2 {
    pub fn initial(request: crate::measure_decision_fixtures::Fixture) -> Self {
        Self {
            actor: request.actor,
            case_id: request.case_id,
            command: request.command,
            material: MeasureDecisionMaterialV2 {
                context: request.material.context,
                support: request.material.support,
                anchor: request.material.anchor,
                predecessors: vec![],
                result_sources: request.material.result_sources,
            },
            history: empty_decision_history(),
            recorded_at: crate::measure_decision_fixtures::at(),
        }
    }

    pub fn confirm(
        previous: &MeasureAdministrativeCapture,
        ancestors: &MeasureRecordHistoryEvidence,
    ) -> Self {
        let prior = &previous.records[0];
        let mut fixture = Self::initial(crate::measure_decision_fixtures::Fixture::single());
        fixture.identities(1);
        fixture.material.context = previous.review.context.clone();
        fixture.command.context = expectation(&fixture.material.context);
        fixture.effects(vec![MeasureEffect::Confirm {
            previous: record_reference(prior),
        }]);
        fixture.material.predecessors = vec![OwnedMeasureRecord::Administrative {
            owner: MeasureAdministrativeRef {
                operation_id: previous.review.command.operation_id,
                capture_digest: previous.capture_digest,
            },
            capture: Box::new(prior.clone()),
        }];
        fixture.material.result_sources = vec![MeasureResultSources {
            id: prior.result.id,
            sources: prior.result.sources.clone(),
        }];
        fixture.history.records = append_administrative(ancestors, previous);
        fixture.recorded_at = previous.recorded_at + Duration::seconds(1);
        fixture
    }

    pub fn identities(&mut self, serial: u128) {
        self.command.operation_id =
            MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + serial));
        self.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(2000 + serial));
    }

    pub fn from_v1(
        previous: &MeasureDecisionGroupCapture,
        ancestors: &MeasureHistoryEvidence,
    ) -> Self {
        let prior = &previous.measures[0];
        let mut fixture = Self::initial(crate::measure_decision_fixtures::Fixture::single());
        fixture.identities(1);
        fixture.material.context = previous.review.material.context.clone();
        fixture.command.context = expectation(&fixture.material.context);
        fixture.effects(vec![MeasureEffect::Confirm {
            previous: reference(prior),
        }]);
        fixture.material.predecessors = vec![OwnedMeasureRecord::Judicial(
            OwnedJudicialMeasure::V1(Box::new(owned(previous))),
        )];
        fixture.material.result_sources = vec![MeasureResultSources {
            id: prior.result.id,
            sources: prior.result.sources.clone(),
        }];
        fixture.history.records.judicial =
            crate::effect_support::append_history(ancestors, previous);
        fixture.recorded_at = previous.recorded_at + Duration::seconds(1);
        fixture
    }

    pub fn next(
        previous: &MeasureDecisionGroupCaptureV2,
        ancestors: &MeasureDecisionRecordHistoryEvidence,
        serial: u128,
    ) -> Self {
        let prior = &previous.measures[0];
        let mut fixture = Self::initial(crate::measure_decision_fixtures::Fixture::single());
        fixture.identities(serial);
        fixture.material.context = previous.review.material.context.clone();
        fixture.command.context = expectation(&fixture.material.context);
        fixture.effects(vec![MeasureEffect::Confirm {
            previous: reference_v2(prior),
        }]);
        fixture.material.predecessors = vec![owned_v2(previous, 0)];
        fixture.material.result_sources = vec![MeasureResultSources {
            id: prior.result.id,
            sources: prior.result.sources.clone(),
        }];
        fixture.history = append_v2(ancestors, previous);
        fixture.recorded_at = previous.recorded_at + Duration::seconds(1);
        fixture
    }

    pub fn effects(&mut self, effects: Vec<MeasureEffect>) {
        self.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    }

    pub fn prepare(&self) -> Result<CheckedMeasureDecisionReviewV2, ApplicationError> {
        prepare_measure_decision_with_record_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            self.material.clone(),
            &self.history,
        )
    }

    pub fn capture(&self) -> MeasureDecisionGroupCaptureV2 {
        self.prepare()
            .unwrap()
            .into_group_capture(&Hasher, self.recorded_at)
            .unwrap()
    }
}
