#[path = "vector_administrative.rs"]
mod administrative_literal;
#[path = "vector_group.rs"]
mod group_literal;
#[path = "vector_measure.rs"]
mod measure_literal;
#[path = "vector_review.rs"]
mod review_literal;

use crate::measure_correction_vector_support::correction_capture;
use crate::measure_decision_vector_support::{assert_vector, capture, Hasher as ScalarHasher};
use crate::record_decision_support::*;
use time::OffsetDateTime;
use uuid::Uuid;

// Scalar fixture: case1, actor3, G1 operation4/decision5, measure9/R1;
// A1 operation10 corrects conditions C to K; G2 operation11/decision12 confirms
// C1/R2 at Unix0+11ns. A2 operation13 corrects M2/R3 to Z at Unix0+12ns.
fn confirmed() -> (
    MeasureDecisionGroupCaptureV2,
    MeasureDecisionRecordHistoryEvidence,
) {
    let base = capture(false);
    let correction = correction_capture();
    let judicial = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin: measure_group_origin(
                &ScalarHasher,
                &base,
                &MeasureHistoryEvidence { groups: vec![] },
            )
            .unwrap(),
            capture: base.clone(),
        }],
    };
    let mut records = MeasureRecordHistoryEvidence {
        judicial,
        administrative: vec![],
    };
    let origin =
        measure_administrative_origin_with_history(&ScalarHasher, &correction, &records).unwrap();
    records.administrative.push(MeasureAdministrativeEvidence {
        origin,
        capture: correction.clone(),
    });
    let history = MeasureDecisionRecordHistoryEvidence {
        records,
        decisions: vec![],
    };
    let mut command = base.review.command.clone();
    command.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(11));
    command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(12));
    command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Confirm {
            previous: record_reference(&correction.records[0]),
        },
    ]))
    .unwrap();
    let material = MeasureDecisionMaterialV2 {
        context: base.review.material.context.clone(),
        support: base.review.material.support.clone(),
        anchor: None,
        predecessors: vec![OwnedMeasureRecord::Administrative {
            owner: MeasureAdministrativeRef {
                operation_id: correction.review.command.operation_id,
                capture_digest: correction.capture_digest,
            },
            capture: Box::new(correction.records[0].clone()),
        }],
        result_sources: vec![MeasureResultSources {
            id: correction.review.result.id,
            sources: correction.review.result.sources.clone(),
        }],
    };
    let group = prepare_measure_decision_with_record_history(
        &ScalarHasher,
        &base.review.actor,
        base.review.case_id,
        command,
        material,
        &history,
    )
    .unwrap()
    .into_group_capture(
        &ScalarHasher,
        OffsetDateTime::from_unix_timestamp_nanos(11).unwrap(),
    )
    .unwrap();
    (group, history)
}

#[test]
fn corrected_confirmation_review_matches_independent_mdpr2_literal() {
    let (group, _) = confirmed();
    assert_vector(
        &measure_decision_review_v2_bytes(&group.review).unwrap(),
        review_literal::REVIEW_LEN,
        review_literal::REVIEW_HEX,
    );
}

#[test]
fn corrected_confirmation_measure_matches_independent_mmcr2_literal() {
    let (group, _) = confirmed();
    assert_vector(
        &measure_capture_v2_bytes(&group.measures[0]).unwrap(),
        measure_literal::MEASURE_LEN,
        measure_literal::MEASURE_HEX,
    );
}

#[test]
fn corrected_confirmation_group_matches_independent_mdgr2_literal() {
    let (group, history) = confirmed();
    assert_vector(
        &measure_decision_group_v2_bytes(&group).unwrap(),
        group_literal::GROUP_LEN,
        group_literal::GROUP_HEX,
    );
    measure_decision_group_v2_matches(&ScalarHasher, &group, &history).unwrap();
}

#[test]
fn correction_after_m2_preserves_magr1_with_the_actual_v2_owner_commitment() {
    let (group, mut history) = confirmed();
    let origin = measure_group_origin_v2(&ScalarHasher, &group, &history).unwrap();
    history.decisions.push(MeasureGroupEvidenceV2 {
        origin,
        capture: group.clone(),
    });
    let result = &group.measures[0].result;
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(13)),
        target: reference_v2(&group.measures[0]),
        context: group.review.command.context,
        reason: note("R"),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            note("Z"),
            result.values.validity().clone(),
            note("U"),
        )),
    };
    let correction = prepare_measure_administrative_record_with_decision_history(
        &ScalarHasher,
        &group.review.actor,
        group.review.case_id,
        command,
        group.review.material.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(
        &ScalarHasher,
        OffsetDateTime::from_unix_timestamp_nanos(12).unwrap(),
    )
    .unwrap();
    assert_vector(
        &measure_administrative_capture_bytes(&correction).unwrap(),
        administrative_literal::ADMINISTRATIVE_LEN,
        administrative_literal::ADMINISTRATIVE_HEX,
    );
    assert_eq!(
        correction.review.result.last_judicial.reference,
        reference_v2(&group.measures[0])
    );
    measure_administrative_capture_with_decision_history_matches(
        &ScalarHasher,
        &correction,
        &history,
    )
    .unwrap();
}
