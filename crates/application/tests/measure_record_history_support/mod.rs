pub use crate::correction_support::*;

use application::identity::Principal;
use application::precautionary_hearings::PrecautionaryContext;
use application::ApplicationError;
use domain::cases::CaseId;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn empty() -> MeasureRecordHistoryEvidence {
    MeasureRecordHistoryEvidence {
        judicial: crate::effect_support::empty_history(),
        administrative: vec![],
    }
}

pub fn record_reference(record: &MeasureAdministrativeRecordCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        record.result.id,
        record.result.revision,
        record.capture_digest,
    )
}

pub fn append_administrative(
    history: &MeasureRecordHistoryEvidence,
    capture: &MeasureAdministrativeCapture,
) -> MeasureRecordHistoryEvidence {
    let origin = measure_administrative_origin_with_history(&Hasher, capture, history).unwrap();
    let mut result = history.clone();
    result.administrative.push(MeasureAdministrativeEvidence {
        origin,
        capture: capture.clone(),
    });
    result
}

#[derive(Clone)]
pub struct RecordFixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureAdministrativeCommand,
    pub context: PrecautionaryContext,
    pub history: MeasureRecordHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}

impl RecordFixture {
    pub fn initial() -> Self {
        Self::from_first(CorrectionFixture::initial())
    }

    pub fn from_first(first: CorrectionFixture) -> Self {
        Self {
            actor: first.actor,
            case_id: first.case_id,
            command: first.command,
            context: first.context,
            history: MeasureRecordHistoryEvidence {
                judicial: first.history,
                administrative: vec![],
            },
            recorded_at: first.recorded_at,
        }
    }

    pub fn next(
        previous: &MeasureAdministrativeCapture,
        ancestors: &MeasureRecordHistoryEvidence,
        serial: u128,
    ) -> Self {
        let prior = &previous.records[0];
        let supervision = match prior.result.values.supervision() {
            MeasureSupervision::Known { statement, .. } => statement,
            MeasureSupervision::Unknown { reason } => reason,
        };
        Self {
            actor: previous.review.actor.clone(),
            case_id: previous.review.case_id,
            command: MeasureAdministrativeCommand {
                operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(
                    500 + serial,
                )),
                target: record_reference(prior),
                context: expectation(&previous.review.context),
                reason: note("Correct another transcribed condition"),
                action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                    note(&format!("Corrected condition {serial}")),
                    prior.result.values.validity().clone(),
                    supervision.clone(),
                )),
            },
            context: previous.review.context.clone(),
            history: append_administrative(ancestors, previous),
            recorded_at: previous.recorded_at + Duration::seconds(1),
        }
    }

    pub fn prepare(&self) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
        prepare_measure_record_correction_with_history(
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
