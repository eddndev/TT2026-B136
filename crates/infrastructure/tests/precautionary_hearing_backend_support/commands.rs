use super::*;

pub fn input(values: &PrecautionaryHearingValues) -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: values.purpose(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: values.participants().to_vec(),
        scheduling_basis: values.scheduling_basis().clone(),
        review_targets: values.review_targets().to_vec(),
    }
}

pub fn replacement(previous: &PrecautionaryHearingStoredOperation) -> PrecautionaryHearingCommand {
    let prior = &previous.capture;
    let mut values = input(&prior.review.resolved_values);
    values.venue = HearingVenue::new("Replacement court").unwrap();
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: prior.review.command.hearing_id,
        change: PrecautionaryHearingChange::Replace {
            expected_revision: prior.review.result_revision,
            expected_capture_digest: prior.capture_digest,
            context: expectation(&prior.review.observed_context),
            values: PrecautionaryHearingValues::new(values).unwrap(),
            reason: HearingNote::new("Changed communicated venue").unwrap(),
        },
    }
}

pub fn cancellation(previous: &PrecautionaryHearingStoredOperation) -> PrecautionaryHearingCommand {
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: previous.capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Cancel {
            expected_revision: previous.capture.review.result_revision,
            expected_capture_digest: previous.capture.capture_digest,
            reason: HearingNote::new("Declared cancellation").unwrap(),
        },
    }
}
