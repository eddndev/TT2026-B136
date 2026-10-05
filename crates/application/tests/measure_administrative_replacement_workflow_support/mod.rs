pub use crate::replacement_support::*;
pub use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
pub use domain::hearings::HearingSupportRef;
pub use std::sync::Arc;
#[path = "../measure_administrative_workflow_support/ports.rs"]
mod ports;
pub use ports::*;

pub fn now() -> OffsetDateTime {
    crate::measure_decision_fixtures::at() + time::Duration::seconds(100)
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
                DocumentId::from_uuid(uuid::Uuid::from_u128(91)),
                DocumentVersion::initial(),
                "resolution.pdf",
                b"Declared measure decision support",
            )
            .unwrap();
        let mut judicial = crate::measure_decision_fixtures::Fixture::single();
        let support = &mut judicial.material.support;
        support.reference = DocumentVersionRef {
            id: record.id,
            version: record.version,
        };
        support.digest = record.digest;
        support.name = record.name.clone();
        let mut input = crate::measure_decision_fixtures::decision_input(&judicial.command.values);
        input.support = HearingSupportRef::new(support.reference, support.digest);
        judicial.command.values = MeasureDecisionValues::new(input);
        let group = judicial.capture();
        let prior = CorrectionFixture::from_group(
            &group,
            &crate::effect_support::empty_history(),
            group.measures[0].result.id,
        );
        let fixture = ReplacementFixture::from_correction(prior);
        let material = MeasureAdministrativeReady {
            context: fixture.context,
            support_record: record,
            target_head: fixture.command.target,
            replacement_subject: Some(fixture.subject),
            dependency_inventory: MeasureAdministrativeDependencyInventory {
                records: fixture.history.clone(),
                hearings: vec![],
            },
        };
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command: fixture.command,
            material,
            history: fixture.history,
        }
    }
    pub fn checked(&self) -> CheckedMeasureAdministrativeReview {
        prepare_measure_administrative_replacement_with_decision_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            MeasureAdministrativeReplacementMaterial {
                context: self.material.context.clone(),
                subject: self.material.replacement_subject.clone().unwrap(),
            },
            &self.history,
        )
        .unwrap()
    }
    pub fn review(&self) -> MeasureAdministrativeReview {
        self.checked().review().clone()
    }
    pub fn operation(&self, at: OffsetDateTime) -> MeasureAdministrativeStoredOperation {
        let capture = self.checked().into_capture(&Hasher, at).unwrap();
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
        let material = self.material.clone();
        self.returning(MeasureAdministrativePreparation::Ready(Box::new(material)))
    }
    pub fn replay_store(&self, operation: MeasureAdministrativeStoredOperation) -> MockStore {
        self.returning(MeasureAdministrativePreparation::Replay(Box::new(
            operation,
        )))
    }
    fn returning(&self, result: MeasureAdministrativePreparation) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case = self.case_id;
        let command = self.command.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |a, c, cmd, _| {
                assert_eq!((a, c, cmd), (&actor, case, &command));
                Ok(result)
            });
        store
    }
}
