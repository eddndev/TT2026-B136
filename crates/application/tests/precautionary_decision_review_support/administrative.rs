use super::*;
use application::identity::Principal;
use application::ApplicationError;
use domain::cases::CaseId;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn append_administrative_decision_history(
    history: &MeasureDecisionRecordHistoryEvidence,
    capture: &MeasureAdministrativeCapture,
) -> MeasureDecisionRecordHistoryEvidence {
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, capture, history).unwrap();
    let mut result = history.clone();
    result
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: capture.clone(),
        });
    result
}

#[derive(Clone)]
pub struct AdministrativeFixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureAdministrativeCommand,
    pub context: PrecautionaryContext,
    pub history: MeasureDecisionRecordHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}

impl AdministrativeFixture {
    pub fn after(
        group: &MeasureDecisionGroupCaptureV2,
        ancestors: &MeasureDecisionRecordHistoryEvidence,
        serial: u128,
    ) -> Self {
        let prior = &group.measures[0];
        let supervision = match prior.result.values.supervision() {
            MeasureSupervision::Known { statement, .. } => statement,
            MeasureSupervision::Unknown { reason } => reason,
        };
        Self {
            actor: group.review.actor.clone(),
            case_id: group.review.case_id,
            command: MeasureAdministrativeCommand {
                operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(
                    3000 + serial,
                )),
                target: reference_v2(prior),
                context: expectation(&group.review.material.context),
                reason: note("Correct the transcribed later judicial terms"),
                action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                    note("Corrected after the actual M2 capture"),
                    prior.result.values.validity().clone(),
                    supervision.clone(),
                )),
            },
            context: group.review.material.context.clone(),
            history: append_v2(ancestors, group),
            recorded_at: group.recorded_at + Duration::seconds(1),
        }
    }

    pub fn prepare(&self) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
        prepare_measure_administrative_record_with_decision_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            self.context.clone(),
            &self.history,
        )
    }

    pub fn capture(&self) -> MeasureAdministrativeCapture {
        self.prepare()
            .unwrap()
            .into_capture(&Hasher, self.recorded_at)
            .unwrap()
    }
}
