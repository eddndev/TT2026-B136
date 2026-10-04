mod literals;

pub use literals::*;

use crate::measure_decision_vector_support::{capture, Hasher};
use application::measure_corrections::{
    prepare_measure_record_correction, MeasureAdministrativeAction, MeasureAdministrativeCapture,
    MeasureAdministrativeCommand,
};
use application::precautionary_hearings::PrecautionaryContextExpectation;
use application::precautionary_measures::{
    measure_group_origin, MeasureGroupEvidence, MeasureHistoryEvidence,
};
use domain::hearings::HearingNote;
use domain::precautionary_hearings::PrecautionaryMeasureRef;
use domain::precautionary_measures::{MeasureCorrectionOperationId, MeasureCorrectionValues};
use time::OffsetDateTime;
use uuid::Uuid;

// Scalar base: case1, actor3/email bytes 72 c3 a9/Litigator, decision5/operation4,
// measure9/R1, subject7/R1, support6/V2, context0+7ns, subject0+8ns, group0+9ns.
// Correct conditions C to K with administrative operation10 and reason R;
// validity and Unknown supervision text U are retained. Capture is Unix0+10ns.
pub fn correction_capture() -> MeasureAdministrativeCapture {
    let group = capture(false);
    let selected = &group.measures[0];
    let target = PrecautionaryMeasureRef::new(
        selected.result.id,
        selected.result.revision,
        selected.capture_digest,
    );
    let actor = group.review.actor.clone();
    let case_id = group.review.case_id;
    let context = group.review.material.context.clone();
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(10)),
        target,
        context: PrecautionaryContextExpectation {
            administration_revision: context.material().administration.revision,
            stage_revision: context.material().stage.stage_revision(),
            context_digest: context.digest(&Hasher),
        },
        reason: HearingNote::new("R").unwrap(),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            HearingNote::new("K").unwrap(),
            selected.result.values.validity().clone(),
            HearingNote::new("U").unwrap(),
        )),
    };
    let origin =
        measure_group_origin(&Hasher, &group, &MeasureHistoryEvidence { groups: vec![] }).unwrap();
    let history = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin,
            capture: group,
        }],
    };
    prepare_measure_record_correction(&Hasher, &actor, case_id, command, context, &history)
        .unwrap()
        .into_capture(
            &Hasher,
            OffsetDateTime::from_unix_timestamp_nanos(10).unwrap(),
        )
        .unwrap()
}
