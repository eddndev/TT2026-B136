pub use application::precautionary_measures::*;
pub use domain::precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef};
pub use domain::precautionary_measures::*;

use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::identity::Principal;
use application::precautionary_hearings::{PrecautionaryContext, PrecautionaryContextExpectation};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{
    DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::hearings::{HearingNote, HearingSupportRef};
use domain::identity::{Role, UserId};
use domain::participants::DirectoryStatus;
use domain::procedural_time::DeclaredProceduralTime;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{context_support, measure_source_support, participant_support};
pub use context_support::Hasher;

pub fn at() -> OffsetDateTime {
    context_support::at() + Duration::seconds(60)
}

pub fn id(value: u128) -> MeasureId {
    MeasureId::from_uuid(Uuid::from_u128(value))
}

pub fn expectation(context: &PrecautionaryContext) -> PrecautionaryContextExpectation {
    PrecautionaryContextExpectation {
        administration_revision: context.material().administration.revision,
        stage_revision: context.material().stage.stage_revision(),
        context_digest: context.digest(&Hasher),
    }
}

pub fn decision_input(values: &MeasureDecisionValues) -> MeasureDecisionValuesInput {
    MeasureDecisionValuesInput {
        authority: values.authority().clone(),
        declared_at: values.declared_at().clone(),
        justification: values.justification().clone(),
        support: values.support(),
        locator: values.locator().clone(),
    }
}

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureDecisionCommand,
    pub material: MeasureDecisionMaterial,
}

impl Fixture {
    pub fn single() -> Self {
        let context = PrecautionaryContext::new(&Hasher, context_support::initial()).unwrap();
        let source = measure_source_support::Fixture::typed(false, DirectoryStatus::Archived);
        let support = StageSupportSnapshot {
            reference: DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(91)),
                version: DocumentVersion::initial(),
            },
            digest: Sha256Digest::from_array([11; 32]),
            name: "resolution.pdf".into(),
            format: StageDocumentFormat::Pdf,
            policy: StageFormatPolicy::PdfDocxV1,
        };
        Self {
            actor: Principal {
                id: UserId::from_uuid(Uuid::from_u128(2)),
                email: "recording@example.test".into(),
                role: Role::Litigator,
            },
            case_id: context.material().case_id,
            command: MeasureDecisionCommand {
                operation_id: MeasureDecisionOperationId::from_uuid(Uuid::from_u128(100)),
                decision_id: MeasureDecisionId::from_uuid(Uuid::from_u128(110)),
                context: expectation(&context),
                values: MeasureDecisionValues::new(MeasureDecisionValuesInput {
                    authority: HearingNote::new("Declared authority").unwrap(),
                    declared_at: MeasureTime::new(
                        DeclaredProceduralTime::unknown(),
                        Some(HearingNote::new("Decision time not stated").unwrap()),
                    )
                    .unwrap(),
                    justification: HearingNote::new("Declared reasons").unwrap(),
                    support: HearingSupportRef::new(support.reference, support.digest),
                    locator: HearingNote::new("Page 2").unwrap(),
                }),
                anchor: None,
                outcome: MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                    MeasureEffect::Impose(MeasureProposal {
                        id: id(70),
                        values: source.values,
                    }),
                ]))
                .unwrap(),
            },
            material: MeasureDecisionMaterial {
                context,
                support,
                anchor: None,
                predecessors: vec![],
                result_sources: vec![MeasureResultSources {
                    id: id(70),
                    sources: source.sources,
                }],
            },
        }
    }

    pub fn multiple() -> Self {
        let mut fixture = Self::single();
        let mut source = measure_source_support::Fixture::manual(DirectoryStatus::Active);
        participant_support::manual_mut(source.sources.supervisor.as_mut().unwrap()).id =
            participant_support::reference(8, 3).id();
        source.values = MeasureValues::new(measure_source_support::input(&source.sources));
        fixture.add_imposition(80, source.values, source.sources);
        fixture
    }

    pub fn no_change() -> Self {
        let mut fixture = Self::single();
        fixture.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(
                HearingNote::new("No measure change stated").unwrap(),
            ))
            .unwrap();
        fixture.material.result_sources.clear();
        fixture
    }

    pub fn add_imposition(&mut self, value: u128, values: MeasureValues, sources: MeasureSources) {
        let mut effects = self.command.outcome.changes().unwrap().to_vec();
        effects.push(MeasureEffect::Impose(MeasureProposal {
            id: id(value),
            values,
        }));
        self.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
        self.material.result_sources.push(MeasureResultSources {
            id: id(value),
            sources,
        });
    }

    pub fn replace_values(&mut self, measure: MeasureId, values: MeasureValues) {
        let mut effects = self.command.outcome.changes().unwrap().to_vec();
        for effect in &mut effects {
            if let MeasureEffect::Impose(proposal) = effect {
                if proposal.id == measure {
                    proposal.values = values.clone();
                }
            }
        }
        self.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    }

    pub fn prepare(self) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
        prepare_measure_decision_capture(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command,
            self.material,
        )
    }

    pub fn capture(self) -> MeasureDecisionGroupCapture {
        self.prepare()
            .unwrap()
            .into_group_capture(&Hasher, at())
            .unwrap()
    }
}

pub fn reference(measure: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        measure.result.id,
        measure.result.revision,
        measure.capture_digest,
    )
}

pub fn owned(group: &MeasureDecisionGroupCapture) -> OwnedMeasureMaterial {
    OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: group.decision.operation_id,
            decision_id: group.decision.decision_id,
            group_digest: group.capture_digest,
        },
        capture: group.measures[0].clone(),
    }
}

/// Rehashes public copies without repairing their independently recorded facts.
pub fn refresh_digests(group: &mut MeasureDecisionGroupCapture) {
    group.review.submission_digest = Hasher.hash_bytes(
        &measure_decision_submission_bytes(
            &group.review.actor,
            group.review.case_id,
            &group.review.command,
        )
        .unwrap(),
    );
    group.review.review_digest =
        Hasher.hash_bytes(&measure_decision_review_bytes(&group.review).unwrap());
    group.decision.capture_digest =
        Hasher.hash_bytes(&measure_decision_capture_bytes(&group.decision).unwrap());
    for measure in &mut group.measures {
        measure.capture_digest = Hasher.hash_bytes(&measure_capture_bytes(measure).unwrap());
    }
    refresh_group_digest(group);
}

pub fn refresh_group_digest(group: &mut MeasureDecisionGroupCapture) {
    group.capture_digest = Hasher.hash_bytes(&measure_decision_group_bytes(group).unwrap());
}

pub fn assert_invalid(group: &MeasureDecisionGroupCapture) {
    assert!(measure_decision_group_matches(&Hasher, group).is_err());
}
