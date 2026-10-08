mod ports;
pub use ports::*;
mod lifecycle;

pub use crate::context_support::Hasher;
use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::identity::Principal;
pub use application::precautionary_hearings::*;
pub use application::precautionary_measures::MeasureDecisionRecordHistoryEvidence;
pub use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::hearings::HearingSupportRef;
pub use domain::precautionary_hearings::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::precautionary_receipt_support::at() + Duration::seconds(2)
}

pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}

pub fn confirmation(review: &PrecautionaryHearingReview) -> PrecautionaryHearingConfirmation {
    PrecautionaryHearingConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: PrecautionaryHearingCommand,
    pub material: PrecautionaryHearingRecordReady,
}

impl Fixture {
    pub fn schedule() -> Self {
        use crate::record_support::{append_administrative, record_reference, RecordFixture};
        let correction = RecordFixture::initial();
        let corrected = correction.capture();
        let history = append_administrative(&correction.history, &corrected);
        let pure = crate::record_review_support::RecordReviewFixture::schedule(
            vec![record_reference(&corrected.records[0])],
            history.clone(),
        );
        let hearing = pure.hearing;
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(90)),
                DocumentVersion::initial(),
                "appointment.pdf",
                b"Declared appointment support",
            )
            .unwrap();
        let mut command = hearing.command;
        let PrecautionaryHearingChange::Schedule { values, .. } = &mut command.change else {
            unreachable!()
        };
        let mut input = crate::record_review_support::values_input(values);
        input.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
            input.scheduling_basis.statement().clone(),
            HearingSupportRef::new(
                DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
            ),
            input.scheduling_basis.locator().clone(),
        );
        *values = PrecautionaryHearingValues::new(input).unwrap();
        Self {
            actor: hearing.actor,
            case_id: hearing.case_id,
            command,
            material: PrecautionaryHearingRecordReady {
                observed_context: hearing.context,
                history: None,
                selected_sources: Some(PrecautionaryHearingSelectedSources {
                    participants: hearing.sources.participants,
                    support_record: record,
                }),
                record_history: MeasureDecisionRecordHistoryEvidence {
                    records: history,
                    decisions: vec![],
                },
            },
        }
    }

    pub fn checked(&self) -> CheckedPrecautionaryHearingReview {
        let predecessor = self
            .material
            .history
            .as_ref()
            .and_then(|h| h.captures.last());
        let sources = match &self.material.selected_sources {
            Some(selected) => PrecautionaryHearingSources {
                participants: selected.participants.clone(),
                support: StageSupportSnapshot {
                    reference: DocumentVersionRef {
                        id: selected.support_record.id,
                        version: selected.support_record.version,
                    },
                    digest: selected.support_record.digest,
                    name: selected.support_record.name.clone(),
                    format: StageDocumentFormat::Pdf,
                    policy: StageFormatPolicy::PdfDocxV1,
                },
            },
            None => predecessor.unwrap().review.sources.clone(),
        };
        prepare_precautionary_hearing_with_decision_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            PrecautionaryHearingDecisionPreparationMaterial {
                observed_context: self.material.observed_context.clone(),
                sources,
                predecessor,
                decision_history: &self.material.record_history,
            },
        )
        .unwrap()
    }

    pub fn review(&self) -> PrecautionaryHearingReview {
        self.checked().review().clone()
    }

    pub fn operation(
        &self,
        recorded_at: OffsetDateTime,
    ) -> PrecautionaryHearingRecordStoredOperation {
        let capture = self.checked().into_capture(&Hasher, recorded_at).unwrap();
        assert!(self.material.history.is_none());
        let history = PrecautionaryHearingRecordHistoryEvidence {
            origin: precautionary_hearing_origin_with_decision_history(
                &Hasher,
                &capture,
                &self.material.record_history,
            )
            .unwrap(),
            captures: vec![capture.clone()],
            record_history: self.material.record_history.clone(),
        };
        PrecautionaryHearingRecordStoredOperation { capture, history }
    }

    pub fn store(&self) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case_id = self.case_id;
        let command = self.command.clone();
        let material = self.material.clone();
        store
            .expect_prepare()
            .times(1)
            .withf(move |a, c, cmd, _| *a == actor && *c == case_id && *cmd == command)
            .return_once(move |_, _, _, _| {
                Ok(PrecautionaryHearingRecordPreparation::Ready(Box::new(
                    material,
                )))
            });
        store
    }
}

mod extended;
pub use extended::*;
mod admission;
mod authorization;
mod bounds;
mod clocks;
mod evidence;
mod history;
mod replay;
mod validity;
