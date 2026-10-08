mod impose;
mod no_change;

pub use impose::*;
pub use no_change::*;

use application::case_stages::{CaseStageEntry, StageSupportSnapshot};
use application::cases::{
    case_administration_digest, CaseActorSnapshot, CaseAdministrationSnapshot,
    CaseInitialStageRegistration, CaseRevision, CaseStageRevision, InitialCaseStage,
    PenalCaseCreation, PenalCaseProfile,
};
use application::documents::{StageDocumentFormat, StageFormatPolicy};
use application::identity::Principal;
use application::participants::ParticipantActorSnapshot;
use application::precautionary_hearings::{
    PrecautionaryContext, PrecautionaryContextExpectation, PrecautionaryContextMaterial,
};
use application::precautionary_measures::*;
use application::typed_participants::{subject_digest, SubjectSnapshot};
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::{
    DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::hearings::{HearingNote, HearingSupportRef};
use domain::identity::{Role, UserId};
use domain::precautionary_hearings::MeasureId;
use domain::precautionary_measures::*;
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{
    CaseSubjectId, Declared, ParticipantEvidenceLocator, ParticipantText, SubjectRevision,
    SubjectRevisionRef, SubjectValues,
};
use domain::DomainError;
use std::io::Read;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Hasher;

// Deterministic dependency for commitment wiring; these are not SHA-256 vectors.
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut bytes = [0u8; 32];
        for (index, byte) in data.iter().enumerate() {
            let slot = index % bytes.len();
            bytes[slot] = bytes[slot].wrapping_add(*byte).wrapping_add(index as u8);
        }
        Sha256Digest::from_array(bytes)
    }

    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| DomainError::StreamRead {
                message: error.to_string(),
            })?;
        Ok(self.hash_bytes(&bytes))
    }
}

fn at(nanos: i128) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap()
}

fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

fn unknown() -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note("U"))).unwrap()
}

fn context(case_id: CaseId) -> PrecautionaryContext {
    let values = PenalCaseCreation::new(
        CaseMetadata::new("A", "B").unwrap(),
        PenalCaseProfile::new("N", "P", "J", "C", &["O"], None, None).unwrap(),
    )
    .into_values();
    let administration = CaseAdministrationSnapshot {
        case_id,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(&Hasher, &values),
        values,
        changed_at: at(7),
        changed_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(2)),
            email: "x".into(),
        },
    };
    let material = PrecautionaryContextMaterial {
        case_id,
        stage: CaseStageEntry::Initial(CaseInitialStageRegistration {
            case_id,
            stage_revision: CaseStageRevision::FIRST,
            administration_revision: CaseRevision::FIRST,
            stage: InitialCaseStage::Investigation,
            administration_digest: administration.values_digest,
            recorded_at: administration.changed_at,
            recorded_by: administration.changed_by.clone(),
        }),
        stage_administration: administration.clone(),
        administration,
    };
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

fn document(id: u128, version: u32) -> DocumentVersionRef {
    DocumentVersionRef {
        id: DocumentId::from_uuid(Uuid::from_u128(id)),
        version: DocumentVersion::new(version).unwrap(),
    }
}

fn measure(case_id: CaseId) -> (MeasureValues, MeasureSources) {
    let values = SubjectValues::institutional_body(
        ParticipantText::new("S").unwrap(),
        Declared::Known(ParticipantText::new("I").unwrap()),
        ParticipantEvidenceLocator::new(document(8, 1), Sha256Digest::from_array([0x33; 32]), "L")
            .unwrap(),
    );
    let subject = SubjectSnapshot {
        case_id,
        id: CaseSubjectId::from_uuid(Uuid::from_u128(7)),
        revision: SubjectRevision::new(1).unwrap(),
        values_digest: subject_digest(&Hasher, &values),
        values,
        changed_at: at(8),
        changed_by: ParticipantActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(2)),
            email: "x".into(),
        },
    };
    let values = MeasureValues::new(MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: subject.id,
            revision: subject.revision,
            values_digest: subject.values_digest,
        },
        kind: MeasureKind::PeriodicAppearance,
        conditions: note("C"),
        validity: MeasureValidity::new(unknown(), note("V"), None).unwrap(),
        supervision: MeasureSupervision::Unknown { reason: note("U") },
    });
    (
        values,
        MeasureSources {
            subject,
            supervisor: None,
        },
    )
}

pub fn capture(no_change: bool) -> MeasureDecisionGroupCapture {
    let case_id = CaseId::from_uuid(Uuid::from_u128(1));
    let actor = Principal {
        id: UserId::from_uuid(Uuid::from_u128(3)),
        email: "r\u{e9}".into(),
        role: Role::Litigator,
    };
    let context = context(case_id);
    let support = HearingSupportRef::new(document(6, 2), Sha256Digest::from_array([0x22; 32]));
    let (values, sources) = measure(case_id);
    let id = MeasureId::from_uuid(Uuid::from_u128(9));
    let outcome = MeasureDecisionOutcome::new(if no_change {
        MeasureDecisionOutcomeInput::NoMeasureChange(note("N"))
    } else {
        MeasureDecisionOutcomeInput::Changes(vec![MeasureEffect::Impose(MeasureProposal {
            id,
            values,
        })])
    })
    .unwrap();
    let command = MeasureDecisionCommand {
        operation_id: MeasureDecisionOperationId::from_uuid(Uuid::from_u128(4)),
        decision_id: MeasureDecisionId::from_uuid(Uuid::from_u128(5)),
        context: PrecautionaryContextExpectation {
            administration_revision: context.material().administration.revision,
            stage_revision: context.material().stage.stage_revision(),
            context_digest: context.digest(&Hasher),
        },
        values: MeasureDecisionValues::new(MeasureDecisionValuesInput {
            authority: note("J"),
            declared_at: unknown(),
            justification: note("Q"),
            support,
            locator: note("L"),
        }),
        anchor: None,
        outcome,
    };
    let material = MeasureDecisionMaterial {
        context,
        support: StageSupportSnapshot {
            reference: support.reference(),
            digest: support.digest(),
            name: "d.pdf".into(),
            format: StageDocumentFormat::Pdf,
            policy: StageFormatPolicy::PdfDocxV1,
        },
        anchor: None,
        predecessors: vec![],
        result_sources: if no_change {
            vec![]
        } else {
            vec![MeasureResultSources { id, sources }]
        },
    };
    prepare_measure_decision_capture(&Hasher, &actor, case_id, command, material)
        .unwrap()
        .into_group_capture(&Hasher, at(9))
        .unwrap()
}

pub fn assert_vector(actual: &[u8], length: usize, expected: &str) {
    let hex: String = actual.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(actual.len(), length);
    assert_eq!(hex, expected);
}
