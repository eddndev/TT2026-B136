use super::*;
use application::precautionary_hearings::*;
use domain::hearings::{HearingModality, HearingTime, HearingVenue};
use domain::precautionary_hearings::*;

#[test]
fn replacement_preserves_an_actual_g2_imposition_as_its_judicial_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, prior, subject, _) = setup_joint(&mut db);
    let mut command = crate::measure_fixture::fresh(&seed.command);
    command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: MeasureId::new(),
            values: prior.group.measures[0].result.values.clone(),
        }),
    ]))
    .unwrap();
    let original = persist_g2(&db, seed.actor.clone(), command);
    let original_row = &original.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        replacement_command(m2_reference(original_row), seed.command.context, &subject),
    );
    let old = &stored.capture.review.result;
    assert_eq!(old.record_root, original_row.result.record_root);
    assert_eq!(old.judicial_origin, original_row.result.judicial_origin);
    assert_eq!(
        old.judicial_origin.operation_id,
        original.origin.operation_id
    );
    assert_eq!(old.last_judicial.reference, m2_reference(original_row));
    assert_eq!(
        old.last_judicial.owner.group_digest,
        original.group.capture_digest
    );
    assert_eq!(
        stored.capture.review.support,
        original.group.decision.support
    );
    assert_joint(&stored, &subject, &original_row.result.values);
    assert!(stored.record_history.records.judicial.groups.is_empty());
    assert_eq!(stored.record_history.decisions.len(), 1);
    assert_eq!(stored.record_history.decisions[0].capture, original.group);
    assert_heads(&db, &seed.actor, &stored);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn replacement_root_survives_real_g2_correction_and_review_of_its_older_valid_record() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, subject, command) = setup_joint(&mut db);
    let replaced = persist(&db, seed.actor.clone(), command);
    let replacement = replacement_row(&replaced);
    let target = row_reference(replacement);
    let later = persist_g2(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm { previous: target }],
        ),
    );
    let actual = &later.group.measures[0];
    assert_eq!(actual.result.record_root, replacement.result.record_root);
    assert_eq!(
        actual.result.judicial_origin,
        replacement.result.judicial_origin
    );
    assert_eq!(actual.result.values, replacement.result.values);
    assert_eq!(actual.result.sources.subject, subject);
    assert_eq!(actual.result.revision.get(), 2);
    assert!(later
        .record_history
        .records
        .administrative
        .iter()
        .any(|a| a.capture == replaced.capture));

    let corrected = persist(
        &db,
        seed.actor.clone(),
        correction(
            m2_reference(actual),
            seed.command.context,
            &actual.result.values,
            "Transcription after the actual new judicial declaration",
        ),
    );
    assert_eq!(corrected.capture.review.result.revision.get(), 3);
    assert_eq!(
        corrected.capture.review.result.record_root,
        replacement.result.record_root
    );
    assert_eq!(
        corrected.capture.review.result.last_judicial.reference,
        m2_reference(actual)
    );
    assert_eq!(
        corrected
            .capture
            .review
            .result
            .last_judicial
            .owner
            .group_digest,
        later.group.capture_digest
    );
    assert_eq!(
        corrected.capture.review.support,
        later.group.decision.support
    );

    let hearing = review_replacement(&db, &seed, target);
    assert_eq!(
        hearing.capture.review.resolved_values.review_targets(),
        &[target]
    );
    assert!(hearing
        .history
        .record_history
        .records
        .administrative
        .iter()
        .any(|a| a.capture == replaced.capture));
    assert!(hearing.history.record_history.decisions.is_empty());
    let workflow = PrecautionaryHearingRecordService::new(
        crate::hearing_fixture::store(&db),
        Arc::new(TestIdentity(seed.actor.clone())),
        processor(),
        Arc::new(FormatCheck(Some(Box::new(|| {
            panic!("historical hearing replay must not admit support")
        })))),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let actual = workflow
        .submit(
            "session",
            db.case,
            hearing.capture.review.command.clone(),
            crate::hearing_fixture::confirmation(&hearing.capture.review),
        )
        .unwrap();
    assert_eq!(actual, hearing);
    reopened(&db, &seed.actor, &replaced);
    reopened(&db, &seed.actor, &corrected);
}

fn persist_g2(
    db: &Fixture,
    actor: Principal,
    command: MeasureDecisionCommand,
) -> MeasureDecisionRecordStoredOperation {
    let service = MeasureDecisionRecordService::new(
        crate::measure_fixture::store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let MeasureDecisionRecordReview::V2(review) = service
        .prepare("session", db.case, command.clone())
        .unwrap()
    else {
        panic!("a fresh decision must use the genuine G2 family")
    };
    let confirmation = MeasureDecisionConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    };
    let MeasureDecisionRecordReceipt::V2(stored) = service
        .submit("session", db.case, command, confirmation)
        .unwrap()
    else {
        panic!("a fresh decision must return its G2 family")
    };
    *stored
}

fn m2_reference(row: &MeasureCaptureV2) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest)
}

fn review_replacement(
    db: &Fixture,
    seed: &Seed,
    target: PrecautionaryMeasureRef,
) -> PrecautionaryHearingRecordStoredOperation {
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: seed.command.context,
            values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                purpose: PrecautionaryHearingPurpose::Review,
                scheduled_at: HearingTime::new(
                    (db.at + time::Duration::days(2))
                        .replace_nanosecond(0)
                        .unwrap(),
                )
                .unwrap(),
                modality: HearingModality::InPerson,
                venue: HearingVenue::new("Declared replacement review").unwrap(),
                note: None,
                participants: vec![],
                scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                    note("Review the exact replacement record"),
                    seed.command.values.support(),
                    note("Page 1"),
                ),
                review_targets: vec![target],
            })
            .unwrap(),
        },
    };
    let service = PrecautionaryHearingRecordService::new(
        crate::hearing_fixture::store(db),
        Arc::new(TestIdentity(seed.actor.clone())),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let review = service
        .prepare("session", db.case, command.clone())
        .unwrap();
    service
        .submit(
            "session",
            db.case,
            command,
            crate::hearing_fixture::confirmation(&review),
        )
        .unwrap()
}
