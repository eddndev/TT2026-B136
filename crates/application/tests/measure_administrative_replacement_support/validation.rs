use crate::replacement_support::*;
use domain::crypto::Sha256Digest;

#[test]
fn replacement_requires_a_fresh_identity_and_one_exact_complete_subject() {
    for change in 0..6 {
        let mut fixture = ReplacementFixture::initial();
        match change {
            0 => fixture.subject.case_id = CaseId::new(),
            1 => fixture.subject.values_digest = Sha256Digest::from_array([7; 32]),
            2 => fixture.subject.changed_by.email = " invalid actor ".into(),
            3 => {
                fixture.subject.changed_at = fixture
                    .subject
                    .changed_at
                    .to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap())
            }
            4 => {
                let MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
                    replacement_id,
                    ..
                } = &mut fixture.command.action
                else {
                    unreachable!()
                };
                *replacement_id = fixture.command.target.id();
            }
            _ => {
                let MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { subject, .. } =
                    &mut fixture.command.action
                else {
                    unreachable!()
                };
                subject.values_digest = Sha256Digest::from_array([8; 32]);
            }
        }
        assert!(
            fixture.prepare().is_err(),
            "accepted inconsistent source or identity {change}"
        );
    }
    let fixture = ReplacementFixture::initial();
    assert!(prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command.clone(),
        fixture.context.clone(),
        &fixture.history,
    )
    .is_err());
}

#[test]
fn fresh_identity_cannot_reuse_an_unselected_sibling_or_replace_immutable_subject_provenance() {
    let mut request = crate::measure_decision_fixtures::Fixture::single();
    let sources = request.material.result_sources[0].sources.clone();
    request.add_imposition(
        10,
        MeasureValues::new(crate::measure_source_support::input(&sources)),
        sources,
    );
    let group = request.capture();
    let fixture = ReplacementFixture::from_correction(CorrectionFixture::from_group(
        &group,
        &crate::effect_support::empty_history(),
        id(70),
    ));
    assert!(fixture.prepare().is_err());

    let mut fixture = ReplacementFixture::initial();
    fixture.subject = fixture.previous().result.sources.subject;
    fixture.subject.changed_by.email = "different-historical-author@example.test".into();
    let MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { subject, .. } =
        &mut fixture.command.action
    else {
        unreachable!()
    };
    *subject = crate::measure_source_support::subject_ref(&fixture.subject);
    assert!(fixture.prepare().is_err());
}

#[test]
fn replacement_capture_cannot_predate_its_new_subject_and_accepts_exact_nanosecond_equality() {
    let mut fixture = ReplacementFixture::initial();
    let source_time = fixture.recorded_at + time::Duration::seconds(10);
    fixture.subject.changed_at = source_time;
    let checked = fixture.prepare().unwrap();
    assert!(checked
        .into_capture(&Hasher, source_time - time::Duration::nanoseconds(1))
        .is_err());
    let capture = fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, source_time)
        .unwrap();
    assert_eq!(capture.recorded_at, source_time);
    assert!(capture
        .records
        .iter()
        .all(|row| row.recorded_at == source_time));
    measure_administrative_capture_with_decision_history_matches(
        &Hasher,
        &capture,
        &fixture.history,
    )
    .unwrap();
}

#[test]
fn terminal_declarations_remain_terminal_and_marked_records_cannot_receive_a_late_replacement() {
    let root = crate::measure_decision_fixtures::Fixture::single().capture();
    let mut terminal = crate::effect_support::LaterFixture::confirm(&root);
    terminal.request.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Revoke {
                previous: reference(&root.measures[0]),
            },
        ]))
        .unwrap();
    let revoked = terminal.clone().capture();
    let fixture = ReplacementFixture::from_correction(CorrectionFixture::from_group(
        &revoked,
        &terminal.evidence,
        revoked.measures[0].result.id,
    ));
    let capture = fixture.capture();
    assert_eq!(
        capture.review.result.last_action,
        MeasureCaptureAction::Revoke
    );
    assert_eq!(
        capture.review.replacement.as_ref().unwrap().last_action,
        MeasureCaptureAction::Revoke
    );

    let fixture = ReplacementFixture::initial();
    let mut marked_command = fixture.command.clone();
    marked_command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        marked_command,
        fixture.context.clone(),
        &fixture.history,
    )
    .unwrap()
    .into_capture(&Hasher, fixture.recorded_at)
    .unwrap();
    let mut retry = fixture.clone();
    retry.command.operation_id = MeasureCorrectionOperationId::new();
    retry.command.target = record_reference(&marked.records[0]);
    retry.history = append(&fixture, &marked);
    assert!(retry.prepare().is_err());
}
