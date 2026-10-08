use crate::record_decision_support::{append_v2, reference_v2, FixtureV2};
use crate::replacement_support::*;

#[test]
fn either_selected_row_resolves_its_complete_joint_owner_and_preserved_judicial_evidence() {
    let fixture = ReplacementFixture::initial();
    let capture = fixture.capture();
    let history = append(&fixture, &capture);
    for selected in &capture.records {
        let checked = resolve_measure_records_with_decision_history(
            &Hasher,
            fixture.case_id,
            &[record_reference(selected)],
            &history,
        )
        .unwrap();
        let resolved = &checked.targets()[0];
        assert_eq!(resolved.reference(), record_reference(selected));
        assert_eq!(resolved.values(), &selected.result.values);
        assert_eq!(resolved.sources(), &selected.result.sources);
        assert_eq!(resolved.validity(), selected.result.validity);
        assert_eq!(resolved.record_root(), selected.result.record_root);
        assert_eq!(resolved.support(), &capture.review.support);
        assert_eq!(resolved.last_judicial().reference(), fixture.command.target);
    }
    let mut missing_sibling = history.clone();
    missing_sibling.records.administrative[0]
        .capture
        .records
        .remove(1);
    let selected = record_reference(&capture.records[0]);
    assert!(resolve_measure_records_with_decision_history(
        &Hasher,
        fixture.case_id,
        &[selected],
        &missing_sibling,
    )
    .is_err());
}

#[test]
fn replacement_identity_survives_a_real_g2_effect_and_a_later_correction() {
    let fixture = ReplacementFixture::initial();
    let replacement = fixture.capture();
    assert_eq!(replacement.records[0].result.id, fixture.replacement_id());
    let judicial = FixtureV2::confirm(&replacement, &fixture.history.records);
    let group = judicial.capture();
    let measure = &group.measures[0];
    assert_eq!(measure.result.id, fixture.replacement_id());
    assert_eq!(measure.result.revision.get(), 2);
    assert_eq!(
        measure.result.record_root,
        replacement.records[0].result.record_root
    );
    assert_eq!(
        measure.result.judicial_origin,
        replacement.records[0].result.judicial_origin
    );
    assert_eq!(measure.result.values, replacement.records[0].result.values);
    assert_eq!(
        measure.result.sources,
        replacement.records[0].result.sources
    );

    let history = append_v2(&judicial.history, &group);
    let mut command = fixture.command.clone();
    command.operation_id = MeasureCorrectionOperationId::new();
    command.target = reference_v2(measure);
    let supervision = match measure.result.values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    command.action = MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
        note("Correct later transcribed conditions"),
        measure.result.values.validity().clone(),
        supervision.clone(),
    ));
    let corrected = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        command,
        fixture.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(&Hasher, group.recorded_at + time::Duration::seconds(1))
    .unwrap();
    assert_eq!(
        corrected.review.result.record_root,
        measure.result.record_root
    );
    assert_eq!(
        corrected.review.result.last_judicial.reference,
        reference_v2(measure)
    );
    assert_eq!(corrected.review.support, group.review.material.support);
    assert_eq!(
        corrected.review.result.values.subject(),
        measure.result.values.subject()
    );
    assert_eq!(corrected.review.result.sources, measure.result.sources);
}
