mod ports;
pub use ports::*;
mod supplemental;

pub use crate::context_support::Hasher;
pub use application::measure_corrections::OwnedMeasureRecord;
pub use application::precautionary_measures::*;
pub use application::ApplicationError;
use application::{
    case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot},
    identity::Principal,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::HearingSupportRef,
    precautionary_measures::MeasureDecisionValues,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub fn now() -> OffsetDateTime {
    crate::measure_decision_fixtures::at() + Duration::seconds(100)
}

pub fn confirmation(review: &MeasureDecisionReviewV2) -> MeasureDecisionConfirmation {
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
    pub material: MeasureDecisionRecordReady,
}

impl Fixture {
    pub fn corrected() -> Self {
        let correction = crate::record_support::RecordFixture::initial();
        let administrative = correction.capture();
        let fixture = crate::record_decision_support::FixtureV2::confirm(
            &administrative,
            &correction.history,
        );
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(92)),
                DocumentVersion::initial(),
                "reviewed-decision.pdf",
                b"Actual support for the later judicial decision",
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
        let mut command = fixture.command;
        command.values = MeasureDecisionValues::new(input);
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command,
            material: MeasureDecisionRecordReady {
                context: fixture.material.context,
                support_record: record,
                anchor: fixture.material.anchor,
                predecessors: fixture.material.predecessors,
                result_sources: fixture.material.result_sources,
                record_history: fixture.history,
            },
        }
    }

    pub fn checked(&self) -> CheckedMeasureDecisionReviewV2 {
        let record = &self.material.support_record;
        prepare_measure_decision_with_record_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            MeasureDecisionMaterialV2 {
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
            },
            &self.material.record_history,
        )
        .unwrap()
    }

    pub fn review(&self) -> MeasureDecisionReviewV2 {
        self.checked().review().clone()
    }

    pub fn operation(&self, at: OffsetDateTime) -> MeasureDecisionRecordStoredOperation {
        let group = self.checked().into_group_capture(&Hasher, at).unwrap();
        let origin =
            measure_group_origin_v2(&Hasher, &group, &self.material.record_history).unwrap();
        MeasureDecisionRecordStoredOperation {
            group,
            origin,
            record_history: self.material.record_history.clone(),
        }
    }

    pub fn legacy_operation(&self) -> MeasureDecisionStoredOperation {
        let entry = &self.material.record_history.records.judicial.groups[0];
        MeasureDecisionStoredOperation {
            group: entry.capture.clone(),
            origin: entry.origin.clone(),
            measure_history: MeasureHistoryEvidence { groups: vec![] },
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
                Ok(MeasureDecisionRecordPreparation::Ready(Box::new(material)))
            });
        store
    }
}
