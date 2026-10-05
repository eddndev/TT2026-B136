#[allow(dead_code, unused_imports)]
#[path = "../measure_decision_vector_support/mod.rs"]
mod judicial;
#[path = "literals.rs"]
mod literals;

use application::measure_corrections::*;
use application::precautionary_hearings::PrecautionaryContextExpectation;
use application::precautionary_measures::{
    measure_group_origin, MeasureGroupEvidence, MeasureHistoryEvidence,
};
use domain::hearings::HearingNote;
use domain::precautionary_hearings::PrecautionaryMeasureRef;
use domain::precautionary_measures::MeasureCorrectionOperationId;
use judicial::{assert_vector, Hasher};
use literals::*;
use time::OffsetDateTime;
use uuid::Uuid;

// Scalar base: case1, actor3/email bytes 72 c3 a9/Litigator, decision5/operation4,
// measure9/R1, subject7/R1, support6/V2, context0+7ns, subject0+8ns, group0+9ns.
// Mark operation10/reason R at Unix0+10ns retains conditions C, validity V and
// Unknown supervision U. Expected bytes are literal independent frames.
fn mark_capture() -> MeasureAdministrativeCapture {
    let group = judicial::capture(false);
    let selected = &group.measures[0];
    let actor = group.review.actor.clone();
    let case_id = group.review.case_id;
    let context = group.review.material.context.clone();
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(10)),
        target: PrecautionaryMeasureRef::new(
            selected.result.id,
            selected.result.revision,
            selected.capture_digest,
        ),
        context: PrecautionaryContextExpectation {
            administration_revision: context.material().administration.revision,
            stage_revision: context.material().stage.stage_revision(),
            context_digest: context.digest(&Hasher),
        },
        reason: HearingNote::new("R").unwrap(),
        action: MeasureAdministrativeAction::MarkEnteredInError,
    };
    let origin =
        measure_group_origin(&Hasher, &group, &MeasureHistoryEvidence { groups: vec![] }).unwrap();
    let evidence = MeasureRecordHistoryEvidence {
        judicial: MeasureHistoryEvidence {
            groups: vec![MeasureGroupEvidence {
                origin,
                capture: group,
            }],
        },
        administrative: vec![],
    };
    prepare_measure_administrative_record_with_history(
        &Hasher, &actor, case_id, command, context, &evidence,
    )
    .unwrap()
    .into_capture(
        &Hasher,
        OffsetDateTime::from_unix_timestamp_nanos(10).unwrap(),
    )
    .unwrap()
}

#[test]
fn mark_instruction_matches_independent_matxn1_vector_without_correction_payload() {
    let capture = mark_capture();
    let review = &capture.review;
    assert_vector(
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)
            .unwrap(),
        INSTRUCTION_LEN,
        INSTRUCTION_HEX,
    );
}

#[test]
fn mark_review_matches_independent_mapr1_vector() {
    assert_vector(
        &measure_administrative_review_bytes(&mark_capture().review).unwrap(),
        REVIEW_LEN,
        REVIEW_HEX,
    );
}

#[test]
fn marked_record_matches_independent_marcr1_vector() {
    let capture = mark_capture();
    assert_eq!(capture.records.len(), 1);
    assert_vector(
        &measure_administrative_record_bytes(&capture.records[0]).unwrap(),
        RECORD_LEN,
        RECORD_HEX,
    );
}

#[test]
fn mark_receipt_matches_independent_magr1_vector() {
    assert_vector(
        &measure_administrative_capture_bytes(&mark_capture()).unwrap(),
        CAPTURE_LEN,
        CAPTURE_HEX,
    );
}
