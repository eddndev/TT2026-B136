pub use crate::measure_decision_fixtures::{expectation, id, owned, reference, Hasher};
pub use application::measure_corrections::*;
pub use application::precautionary_measures::*;
pub use domain::precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef};
pub use domain::precautionary_measures::*;

use application::identity::Principal;
use application::precautionary_hearings::PrecautionaryContext;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::hearings::HearingNote;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{effect_support, measure_decision_fixtures};

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

#[derive(Clone)]
pub struct CorrectionFixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureAdministrativeCommand,
    pub context: PrecautionaryContext,
    pub history: MeasureHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}

impl CorrectionFixture {
    pub fn initial() -> Self {
        let group = measure_decision_fixtures::Fixture::single().capture();
        Self::from_group(
            &group,
            &effect_support::empty_history(),
            group.measures[0].result.id,
        )
    }

    pub fn from_group(
        group: &MeasureDecisionGroupCapture,
        ancestors: &MeasureHistoryEvidence,
        measure_id: MeasureId,
    ) -> Self {
        let prior = group
            .measures
            .iter()
            .find(|row| row.result.id == measure_id)
            .unwrap();
        let supervision = match prior.result.values.supervision() {
            MeasureSupervision::Known { statement, .. } => statement,
            MeasureSupervision::Unknown { reason } => reason,
        };
        Self {
            actor: group.review.actor.clone(),
            case_id: group.review.case_id,
            command: MeasureAdministrativeCommand {
                operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(500)),
                target: reference(prior),
                context: expectation(&group.review.material.context),
                reason: note("Correct the transcribed conditions"),
                action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                    note("Corrected recorded conditions"),
                    prior.result.values.validity().clone(),
                    supervision.clone(),
                )),
            },
            context: group.review.material.context.clone(),
            history: effect_support::append_history(ancestors, group),
            recorded_at: group.recorded_at + Duration::seconds(1),
        }
    }

    pub fn previous_group(&self) -> MeasureDecisionGroupCapture {
        self.history
            .groups
            .iter()
            .find(|entry| {
                entry
                    .capture
                    .measures
                    .iter()
                    .any(|row| reference(row) == self.command.target)
            })
            .unwrap()
            .capture
            .clone()
    }

    pub fn previous(&self) -> MeasureCapture {
        self.previous_group()
            .measures
            .into_iter()
            .find(|row| reference(row) == self.command.target)
            .unwrap()
    }

    pub fn prepare(&self) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
        prepare_measure_record_correction(
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
