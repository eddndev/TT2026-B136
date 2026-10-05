pub use crate::precautionary_receipt_support::Fixture as HearingFixture;
pub use crate::record_support::*;
pub use application::precautionary_hearings::*;
use application::ApplicationError;
pub use domain::precautionary_hearings::*;
use time::OffsetDateTime;

pub fn values_input(values: &PrecautionaryHearingValues) -> PrecautionaryHearingValuesInput {
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

pub fn select_targets(hearing: &mut HearingFixture, targets: Vec<PrecautionaryMeasureRef>) {
    let values = match &mut hearing.command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => values,
        _ => panic!("schedule or replacement expected"),
    };
    let mut input = values_input(values);
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = targets;
    *values = PrecautionaryHearingValues::new(input).unwrap();
}

pub fn set_context(hearing: &mut HearingFixture, context: PrecautionaryContext) {
    match &mut hearing.command.change {
        PrecautionaryHearingChange::Schedule {
            context: expected, ..
        }
        | PrecautionaryHearingChange::Replace {
            context: expected, ..
        } => {
            *expected = crate::precautionary_receipt_support::expectation(&context);
        }
        _ => {}
    }
    hearing.context = context;
}

#[derive(Clone)]
pub struct RecordReviewFixture {
    pub hearing: HearingFixture,
    pub record_history: MeasureRecordHistoryEvidence,
}

impl RecordReviewFixture {
    pub fn schedule(
        targets: Vec<PrecautionaryMeasureRef>,
        record_history: MeasureRecordHistoryEvidence,
    ) -> Self {
        let mut hearing = HearingFixture::schedule();
        select_targets(&mut hearing, targets);
        Self {
            hearing,
            record_history,
        }
    }

    pub fn replace(
        predecessor: &PrecautionaryHearingCapture,
        targets: Vec<PrecautionaryMeasureRef>,
        record_history: MeasureRecordHistoryEvidence,
    ) -> Self {
        let mut hearing = HearingFixture::replace(predecessor);
        select_targets(&mut hearing, targets);
        Self {
            hearing,
            record_history,
        }
    }

    pub fn cancel(
        predecessor: &PrecautionaryHearingCapture,
        record_history: MeasureRecordHistoryEvidence,
    ) -> Self {
        Self {
            hearing: HearingFixture::cancel(predecessor),
            record_history,
        }
    }

    pub fn prepare(
        &self,
        predecessor: Option<&PrecautionaryHearingCapture>,
    ) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
        prepare_precautionary_hearing_with_record_history(
            &Hasher,
            &self.hearing.actor,
            self.hearing.case_id,
            self.hearing.command.clone(),
            PrecautionaryHearingRecordPreparationMaterial {
                observed_context: self.hearing.context.clone(),
                sources: self.hearing.sources.clone(),
                predecessor,
                record_history: &self.record_history,
            },
        )
    }

    pub fn capture(
        &self,
        predecessor: Option<&PrecautionaryHearingCapture>,
        at: OffsetDateTime,
    ) -> PrecautionaryHearingCapture {
        self.prepare(predecessor)
            .unwrap()
            .into_capture(&Hasher, at)
            .unwrap()
    }
}
