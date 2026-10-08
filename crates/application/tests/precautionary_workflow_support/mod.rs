mod ports;
pub use ports::*;
mod authorization;
mod bounds;
mod clock_tests;
mod lifecycle;
mod preparation_tests;
mod replay;

pub use crate::context_support::Hasher;
use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::identity::Principal;
pub use application::precautionary_hearings::*;
pub use application::precautionary_measures::MeasureHistoryEvidence;
pub use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::hearings::HearingSupportRef;
pub use domain::precautionary_hearings::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::receipt_support::at() + Duration::seconds(2)
}

pub fn empty_history() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn confirmation(review: &PrecautionaryHearingReview) -> PrecautionaryHearingConfirmation {
    PrecautionaryHearingConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
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

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: PrecautionaryHearingCommand,
    pub material: PrecautionaryHearingReady,
}

impl Fixture {
    pub fn schedule() -> Self {
        let fixture = crate::receipt_support::Fixture::schedule();
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(90)),
                DocumentVersion::initial(),
                "appointment.pdf",
                b"Declared appointment support",
            )
            .unwrap();
        let mut input = crate::receipt_support::values_input();
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
        let mut command = fixture.command;
        command.change = PrecautionaryHearingChange::Schedule {
            context: crate::receipt_support::expectation(&fixture.context),
            values: PrecautionaryHearingValues::new(input).unwrap(),
        };
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command,
            material: PrecautionaryHearingReady {
                observed_context: fixture.context,
                history: None,
                selected_sources: Some(PrecautionaryHearingSelectedSources {
                    participants: fixture.sources.participants,
                    support_record: record,
                }),
                measure_history: empty_history(),
            },
        }
    }

    pub fn checked(&self) -> CheckedPrecautionaryHearingReview {
        let predecessor = self
            .material
            .history
            .as_ref()
            .and_then(|history| history.captures.last());
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
        prepare_precautionary_hearing_with_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            PrecautionaryHearingPreparationMaterial {
                observed_context: self.material.observed_context.clone(),
                sources,
                predecessor,
                measure_history: &self.material.measure_history,
            },
        )
        .unwrap()
    }

    pub fn review(&self) -> PrecautionaryHearingReview {
        self.checked().review().clone()
    }

    pub fn replace(previous: &PrecautionaryHearingStoredOperation) -> Self {
        let mut fixture = Self::schedule();
        let prior = &previous.capture;
        let mut input = values_input(&prior.review.resolved_values);
        input.venue = domain::hearings::HearingVenue::new("Replacement court").unwrap();
        fixture.material.observed_context = crate::receipt_support::later_context();
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(31));
        fixture.command.change = PrecautionaryHearingChange::Replace {
            expected_revision: prior.review.result_revision,
            expected_capture_digest: prior.capture_digest,
            context: crate::receipt_support::expectation(&fixture.material.observed_context),
            values: PrecautionaryHearingValues::new(input).unwrap(),
            reason: domain::hearings::HearingNote::new("Correct the communicated venue").unwrap(),
        };
        fixture.material.history = Some(previous.history.clone());
        fixture.material.measure_history = previous.history.measure_history.clone();
        fixture
    }

    pub fn cancel(previous: &PrecautionaryHearingStoredOperation) -> Self {
        let mut fixture = Self::replace(previous);
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(32));
        fixture.command.change = PrecautionaryHearingChange::Cancel {
            expected_revision: previous.capture.review.result_revision,
            expected_capture_digest: previous.capture.capture_digest,
            reason: domain::hearings::HearingNote::new("Cancel the communicated appointment")
                .unwrap(),
        };
        fixture.material.selected_sources = None;
        fixture
    }

    pub fn operation(&self, recorded_at: OffsetDateTime) -> PrecautionaryHearingStoredOperation {
        let capture = self.checked().into_capture(&Hasher, recorded_at).unwrap();
        let mut history = match &self.material.history {
            Some(history) => history.clone(),
            None => PrecautionaryHearingHistoryEvidence {
                origin: precautionary_hearing_origin_with_measure_history(
                    &Hasher,
                    &capture,
                    &self.material.measure_history,
                )
                .unwrap(),
                captures: vec![],
                measure_history: empty_history(),
            },
        };
        for entry in &self.material.measure_history.groups {
            if let Some(old) = history
                .measure_history
                .groups
                .iter()
                .find(|old| old.origin.operation_id == entry.origin.operation_id)
            {
                assert_eq!(old, entry);
            } else {
                history.measure_history.groups.push(entry.clone());
            }
        }
        history.captures.push(capture.clone());
        precautionary_hearing_history_with_measure_history_matches(
            &Hasher,
            &history.captures,
            &history.origin,
            &history.measure_history,
        )
        .unwrap();
        PrecautionaryHearingStoredOperation { capture, history }
    }

    pub fn replay_store(&self, operation: PrecautionaryHearingStoredOperation) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case_id = self.case_id;
        let command = self.command.clone();
        store
            .expect_prepare()
            .times(1)
            .withf(move |a, c, cmd, _| *a == actor && *c == case_id && *cmd == command)
            .return_once(move |_, _, _, _| {
                Ok(PrecautionaryHearingPreparation::Replay(Box::new(operation)))
            });
        store
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
                Ok(PrecautionaryHearingPreparation::Ready(Box::new(material)))
            });
        store
    }
}

mod full_capture_test;

mod capture_clock_test;
