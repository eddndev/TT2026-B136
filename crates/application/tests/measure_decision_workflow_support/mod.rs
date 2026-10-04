mod ports;
pub use ports::*;
mod authorization;
mod clock_tests;
mod effects;
mod lifecycle;
mod replay;
mod variants;

pub use crate::context_support::Hasher;
use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::identity::Principal;
pub use application::precautionary_measures::*;
pub use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::hearings::HearingSupportRef;
pub use domain::precautionary_measures::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::measure_decision_fixtures::at()
}
pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}

pub fn empty_history() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn confirmation(review: &MeasureDecisionReview) -> MeasureDecisionConfirmation {
    MeasureDecisionConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureDecisionCommand,
    pub material: MeasureDecisionReady,
}

impl Fixture {
    pub fn single() -> Self {
        Self::from_pure(
            crate::measure_decision_fixtures::Fixture::single(),
            empty_history(),
        )
    }

    pub fn no_change() -> Self {
        Self::from_pure(
            crate::measure_decision_fixtures::Fixture::no_change(),
            empty_history(),
        )
    }

    pub fn from_pure(
        mut fixture: crate::measure_decision_fixtures::Fixture,
        evidence: MeasureHistoryEvidence,
    ) -> Self {
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(91)),
                DocumentVersion::initial(),
                "resolution.pdf",
                b"Declared measure decision support",
            )
            .unwrap();
        let mut input = crate::measure_decision_fixtures::decision_input(&fixture.command.values);
        input.support = HearingSupportRef::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
        );
        fixture.command.values = MeasureDecisionValues::new(input);
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command: fixture.command,
            material: MeasureDecisionReady {
                context: fixture.material.context,
                support_record: record,
                anchor: fixture.material.anchor,
                predecessors: fixture.material.predecessors,
                result_sources: fixture.material.result_sources,
                measure_history: evidence,
            },
        }
    }

    pub fn checked(&self) -> CheckedMeasureDecisionReview {
        let record = &self.material.support_record;
        let material = MeasureDecisionMaterial {
            context: self.material.context.clone(),
            support: StageSupportSnapshot {
                reference: DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                digest: record.digest,
                name: record.name.clone(),
                format: StageDocumentFormat::Pdf,
                policy: StageFormatPolicy::PdfDocxV1,
            },
            anchor: self.material.anchor.clone(),
            predecessors: self.material.predecessors.clone(),
            result_sources: self.material.result_sources.clone(),
        };
        prepare_measure_decision_with_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            material,
            &self.material.measure_history,
        )
        .unwrap()
    }

    pub fn review(&self) -> MeasureDecisionReview {
        self.checked().review().clone()
    }

    pub fn operation(&self, recorded_at: OffsetDateTime) -> MeasureDecisionStoredOperation {
        let group = self
            .checked()
            .into_group_capture(&Hasher, recorded_at)
            .unwrap();
        let origin = measure_group_origin(&Hasher, &group, &self.material.measure_history).unwrap();
        MeasureDecisionStoredOperation {
            group,
            origin,
            measure_history: self.material.measure_history.clone(),
        }
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
                Ok(MeasureDecisionPreparation::Ready(Box::new(material)))
            });
        store
    }

    pub fn replay_store(&self, operation: MeasureDecisionStoredOperation) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case_id = self.case_id;
        let command = self.command.clone();
        store
            .expect_prepare()
            .times(1)
            .withf(move |a, c, cmd, _| *a == actor && *c == case_id && *cmd == command)
            .return_once(move |_, _, _, _| {
                Ok(MeasureDecisionPreparation::Replay(Box::new(operation)))
            });
        store
    }
}

mod preparation_tests;
