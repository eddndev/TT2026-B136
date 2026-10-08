pub use application::precautionary_hearings::*;
pub use application::precautionary_measures::*;
use application::ApplicationError;
pub use domain::precautionary_hearings::*;
use time::OffsetDateTime;

pub use crate::context_support::Hasher;
pub use crate::measure_decision_effect_support::LaterFixture as MeasureChangeFixture;
pub use crate::measure_decision_fixtures::Fixture as MeasureFixture;
pub use crate::precautionary_receipt_support::Fixture as HearingFixture;

pub fn empty_history() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn history(group: &MeasureDecisionGroupCapture) -> MeasureHistoryEvidence {
    let empty = empty_history();
    let origin = measure_group_origin(&Hasher, group, &empty).unwrap();
    MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin,
            capture: group.clone(),
        }],
    }
}

pub fn targets(group: &MeasureDecisionGroupCapture) -> Vec<PrecautionaryMeasureRef> {
    group
        .measures
        .iter()
        .map(crate::measure_decision_fixtures::reference)
        .collect()
}

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
        _ => panic!("scheduling or replacement fixture expected"),
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

pub fn refresh(capture: &mut PrecautionaryHearingCapture) {
    use domain::crypto::DocumentHasher;
    capture.review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &capture.review.actor,
            capture.review.case_id,
            &capture.review.command,
            &capture.review.resolved_values,
        )
        .unwrap(),
    );
    capture.review.review_digest =
        Hasher.hash_bytes(&precautionary_hearing_review_bytes(&capture.review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

#[derive(Clone)]
pub struct ReviewFixture {
    pub hearing: HearingFixture,
    pub measure_history: MeasureHistoryEvidence,
}

impl ReviewFixture {
    pub fn schedule(group: &MeasureDecisionGroupCapture) -> Self {
        let mut hearing = HearingFixture::schedule();
        let mut input = crate::precautionary_receipt_support::values_input();
        input.purpose = PrecautionaryHearingPurpose::Review;
        input.review_targets = group
            .measures
            .iter()
            .map(crate::measure_decision_fixtures::reference)
            .collect();
        hearing.command.change = PrecautionaryHearingChange::Schedule {
            context: crate::precautionary_receipt_support::expectation(&hearing.context),
            values: PrecautionaryHearingValues::new(input).unwrap(),
        };
        Self {
            hearing,
            measure_history: history(group),
        }
    }

    pub fn replace(
        predecessor: &PrecautionaryHearingCapture,
        targets: Vec<PrecautionaryMeasureRef>,
        measure_history: MeasureHistoryEvidence,
    ) -> Self {
        let mut hearing = HearingFixture::replace(predecessor);
        select_targets(&mut hearing, targets);
        Self {
            hearing,
            measure_history,
        }
    }

    pub fn cancel(
        predecessor: &PrecautionaryHearingCapture,
        measure_history: MeasureHistoryEvidence,
    ) -> Self {
        Self {
            hearing: HearingFixture::cancel(predecessor),
            measure_history,
        }
    }

    pub fn prepare(
        self,
        predecessor: Option<&PrecautionaryHearingCapture>,
    ) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
        prepare_precautionary_hearing_with_history(
            &Hasher,
            &self.hearing.actor,
            self.hearing.case_id,
            self.hearing.command,
            PrecautionaryHearingPreparationMaterial {
                observed_context: self.hearing.context,
                sources: self.hearing.sources,
                predecessor,
                measure_history: &self.measure_history,
            },
        )
    }

    pub fn capture(
        self,
        predecessor: Option<&PrecautionaryHearingCapture>,
        at: OffsetDateTime,
    ) -> PrecautionaryHearingCapture {
        self.prepare(predecessor)
            .unwrap()
            .into_capture(&Hasher, at)
            .unwrap()
    }
}
