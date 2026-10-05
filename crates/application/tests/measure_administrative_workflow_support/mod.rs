mod lifecycle;
mod ports;
mod replay;
mod support_tests;
mod variants;
pub use crate::decision_review_support::*;
pub use application::ApplicationError;
use application::{documents::DocumentRecord, identity::Principal};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::HearingSupportRef,
};
pub use ports::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::measure_decision_fixtures::at()
}
pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}
pub fn confirmation(review: &MeasureAdministrativeReview) -> MeasureAdministrativeConfirmation {
    MeasureAdministrativeConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureAdministrativeCommand,
    pub material: MeasureAdministrativeReady,
    pub history: MeasureDecisionRecordHistoryEvidence,
}

impl Fixture {
    pub fn single() -> Self {
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(91)),
                DocumentVersion::initial(),
                "resolution.pdf",
                b"Declared measure decision support",
            )
            .unwrap();
        Self::with_support_record(record)
    }

    pub fn with_support_record(record: DocumentRecord) -> Self {
        let mut judicial = crate::measure_decision_fixtures::Fixture::single();
        let mut values = crate::measure_decision_fixtures::decision_input(&judicial.command.values);
        values.support = HearingSupportRef::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
        );
        judicial.command.values = MeasureDecisionValues::new(values);
        judicial.material.support.reference = DocumentVersionRef {
            id: record.id,
            version: record.version,
        };
        judicial.material.support.digest = record.digest;
        judicial.material.support.name = record.name.clone();
        let group = judicial.capture();
        let correction = CorrectionFixture::from_group(
            &group,
            &crate::effect_support::empty_history(),
            group.measures[0].result.id,
        );
        Self::from_record(RecordFixture::from_first(correction), record)
    }

    pub fn from_record(fixture: RecordFixture, support_record: DocumentRecord) -> Self {
        let history = MeasureDecisionRecordHistoryEvidence {
            records: fixture.history,
            decisions: vec![],
        };
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command: fixture.command.clone(),
            material: MeasureAdministrativeReady {
                context: fixture.context,
                support_record,
                target_head: fixture.command.target,
                dependency_inventory: MeasureAdministrativeDependencyInventory {
                    records: history.clone(),
                    hearings: vec![],
                },
            },
            history,
        }
    }

    pub fn checked(&self) -> CheckedMeasureAdministrativeReview {
        prepare_measure_administrative_record_with_decision_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            self.material.context.clone(),
            &self.history,
        )
        .unwrap()
    }

    pub fn review(&self) -> MeasureAdministrativeReview {
        self.checked().review().clone()
    }

    pub fn operation(&self, recorded_at: OffsetDateTime) -> MeasureAdministrativeStoredOperation {
        let capture = self.checked().into_capture(&Hasher, recorded_at).unwrap();
        let origin =
            measure_administrative_origin_with_decision_history(&Hasher, &capture, &self.history)
                .unwrap();
        MeasureAdministrativeStoredOperation {
            capture,
            origin,
            record_history: self.history.clone(),
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
                Ok(MeasureAdministrativePreparation::Ready(Box::new(material)))
            });
        store
    }

    pub fn replay_store(&self, operation: MeasureAdministrativeStoredOperation) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case_id = self.case_id;
        let command = self.command.clone();
        store
            .expect_prepare()
            .times(1)
            .withf(move |a, c, cmd, _| *a == actor && *c == case_id && *cmd == command)
            .return_once(move |_, _, _, _| {
                Ok(MeasureAdministrativePreparation::Replay(Box::new(
                    operation,
                )))
            });
        store
    }
}
