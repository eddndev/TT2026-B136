pub use crate::record_decision_support::*;
pub use crate::record_review_support::{set_context, HearingFixture};
pub use application::precautionary_hearings::*;
pub use domain::precautionary_hearings::*;

use application::ApplicationError;
use time::OffsetDateTime;

mod administrative;
pub use administrative::*;

pub fn judicial_fixture() -> FixtureV2 {
    let first = RecordFixture::initial();
    let administrative = first.capture();
    FixtureV2::confirm(&administrative, &first.history)
}

#[derive(Clone)]
pub struct DecisionReviewFixture {
    pub hearing: HearingFixture,
    pub decision_history: MeasureDecisionRecordHistoryEvidence,
}

impl DecisionReviewFixture {
    pub fn schedule(
        targets: Vec<PrecautionaryMeasureRef>,
        decision_history: MeasureDecisionRecordHistoryEvidence,
    ) -> Self {
        let mut hearing = HearingFixture::schedule();
        crate::record_review_support::select_targets(&mut hearing, targets);
        Self {
            hearing,
            decision_history,
        }
    }

    pub fn replace(
        predecessor: &PrecautionaryHearingCapture,
        targets: Vec<PrecautionaryMeasureRef>,
        decision_history: MeasureDecisionRecordHistoryEvidence,
    ) -> Self {
        let mut hearing = HearingFixture::replace(predecessor);
        crate::record_review_support::select_targets(&mut hearing, targets);
        Self {
            hearing,
            decision_history,
        }
    }

    pub fn cancel(
        predecessor: &PrecautionaryHearingCapture,
        decision_history: MeasureDecisionRecordHistoryEvidence,
    ) -> Self {
        Self {
            hearing: HearingFixture::cancel(predecessor),
            decision_history,
        }
    }

    pub fn prepare(
        &self,
        predecessor: Option<&PrecautionaryHearingCapture>,
    ) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
        prepare_precautionary_hearing_with_decision_history(
            &Hasher,
            &self.hearing.actor,
            self.hearing.case_id,
            self.hearing.command.clone(),
            PrecautionaryHearingDecisionPreparationMaterial {
                observed_context: self.hearing.context.clone(),
                sources: self.hearing.sources.clone(),
                predecessor,
                decision_history: &self.decision_history,
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
