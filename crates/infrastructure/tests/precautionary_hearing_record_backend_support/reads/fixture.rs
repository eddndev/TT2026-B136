use super::*;
use application::precautionary_measures::*;
use domain::hearings::HearingNote;
use domain::precautionary_measures::MeasureEffect;

pub(super) fn chain(
    db: &mut Fixture,
) -> (
    Seed,
    Vec<PrecautionaryHearingRecordStoredOperation>,
    PrecautionaryHearingRecordStoredOperation,
) {
    let seed = setup(db);
    let corrected = crate::administrative_fixture::corrected_reference(&seed.corrected.capture);
    let command = crate::administrative_fixture::effect_command(
        &seed.judicial.group.review.command,
        vec![MeasureEffect::Confirm {
            previous: corrected,
        }],
    );
    let workflow = MeasureDecisionRecordService::new(
        crate::measure_fixture::store(db),
        Arc::new(TestIdentity(seed.actor.clone())),
        crate::measure_fixture::processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let MeasureDecisionRecordReview::V2(review) = workflow
        .prepare("session", db.case, command.clone())
        .unwrap()
    else {
        panic!("fresh mixed decision must be V2")
    };
    let MeasureDecisionRecordReceipt::V2(group) = workflow
        .submit(
            "session",
            db.case,
            command,
            MeasureDecisionConfirmation {
                submission_digest: review.submission_digest,
                review_digest: review.review_digest,
            },
        )
        .unwrap()
    else {
        panic!("fresh mixed receipt must be V2")
    };
    let row = &group.group.measures[0];
    let target =
        PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest);
    let latest = crate::administrative_fixture::persist(
        db,
        seed.actor.clone(),
        crate::administrative_fixture::correction(
            target,
            seed.corrected.capture.review.command.context,
            &row.result.values,
            "Administrative correction after actual M2",
        ),
    );
    let first = persist(db, seed.actor.clone(), seed.command.clone());
    let mut values = crate::hearing_fixture::input(&first.capture.review.resolved_values);
    values.review_targets = vec![target];
    let replacement = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: first.capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Replace {
            expected_revision: first.capture.review.result_revision,
            expected_capture_digest: first.capture.capture_digest,
            context: crate::hearing_fixture::expectation(&first.capture.review.observed_context),
            values: PrecautionaryHearingValues::new(values).unwrap(),
            reason: HearingNote::new("Select the actual M2").unwrap(),
        },
    };
    let replaced = persist(db, seed.actor.clone(), replacement);
    let cancellation = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: replaced.capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Cancel {
            expected_revision: replaced.capture.review.result_revision,
            expected_capture_digest: replaced.capture.capture_digest,
            reason: HearingNote::new("Declared mixed appointment cancellation").unwrap(),
        },
    };
    let cancelled = persist(db, seed.actor.clone(), cancellation);
    let mut other = seed.command.clone();
    other.operation_id = PrecautionaryHearingOperationId::new();
    other.hearing_id = PrecautionaryHearingId::new();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut other.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.review_targets = vec![crate::administrative_fixture::corrected_reference(
        &latest.capture,
    )];
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let other = persist(db, seed.actor.clone(), other);
    (seed, vec![first, replaced, cancelled], other)
}
