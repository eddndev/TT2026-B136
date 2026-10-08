use super::*;

pub(super) fn set_values(hearing: &mut HearingFixture, values: PrecautionaryHearingValues) {
    match &mut hearing.command.change {
        PrecautionaryHearingChange::Schedule {
            values: current, ..
        }
        | PrecautionaryHearingChange::Replace {
            values: current, ..
        } => *current = values,
        _ => panic!("scheduled values required"),
    }
}

pub(super) fn select(hearing: &mut HearingFixture, selected: PrecautionaryMeasureRef) {
    let mut input = crate::precautionary_receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![selected];
    set_values(hearing, PrecautionaryHearingValues::new(input).unwrap());
}

pub(super) fn set_context(hearing: &mut HearingFixture, context: PrecautionaryContext) {
    let expected = crate::precautionary_receipt_support::expectation(&context);
    match &mut hearing.command.change {
        PrecautionaryHearingChange::Schedule { context, .. }
        | PrecautionaryHearingChange::Replace { context, .. } => *context = expected,
        _ => {}
    }
    hearing.context = context;
}

pub(super) fn prepare(
    hearing: &HearingFixture,
    evidence: &MeasureRecordHistoryEvidence,
    predecessor: Option<&PrecautionaryHearingCapture>,
) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
    prepare_precautionary_hearing_with_record_history(
        &Hasher,
        &hearing.actor,
        hearing.case_id,
        hearing.command.clone(),
        PrecautionaryHearingRecordPreparationMaterial {
            observed_context: hearing.context.clone(),
            sources: hearing.sources.clone(),
            predecessor,
            record_history: evidence,
        },
    )
}

pub(super) fn review_fixture(
    capture: &MeasureAdministrativeCapture,
    ancestors: &MeasureRecordHistoryEvidence,
) -> (HearingFixture, MeasureRecordHistoryEvidence) {
    let mut hearing = HearingFixture::schedule();
    select(&mut hearing, record_reference(&capture.records[0]));
    set_context(&mut hearing, capture.review.context.clone());
    (hearing, append_administrative(ancestors, capture))
}

pub(super) fn corrected() -> (RecordFixture, MeasureAdministrativeCapture) {
    let fixture = RecordFixture::initial();
    let capture = fixture.capture();
    (fixture, capture)
}

pub(super) fn mark(
    previous: &MeasureAdministrativeCapture,
    ancestors: &MeasureRecordHistoryEvidence,
) -> MeasureAdministrativeCapture {
    let mut fixture = RecordFixture::next(previous, ancestors, 1);
    fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    prepare_measure_administrative_record_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.context,
        &fixture.history,
    )
    .unwrap()
    .into_capture(&Hasher, fixture.recorded_at)
    .unwrap()
}

pub(super) fn refresh(capture: &mut PrecautionaryHearingCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}
