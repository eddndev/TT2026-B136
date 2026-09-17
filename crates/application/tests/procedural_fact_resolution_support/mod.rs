use application::{
    case_stages::StageSupportSnapshot,
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    documents::{StageDocumentFormat, StageFormatPolicy},
    hearing_results::HearingResultSnapshot,
    procedural_facts::*,
    ApplicationError,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::{DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use std::{cell::RefCell, io::Read};
use uuid::Uuid;

pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn digest(value: u8) -> Sha256Digest {
    Sha256Digest::from_array([value; 32])
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn reference(revision: u32) -> FactResolutionRef {
    FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(42)),
        revision: FactRevision::new(revision).unwrap(),
    }
}
pub fn selection(reference: Option<FactResolutionRef>) -> FactSourceSelection {
    let Some(reference) = reference else {
        return FactSourceSelection::from_values(&ProceduralFactValues::Resolution(Box::new(
            values(FactProvenance::OperatorNote { note: text("Note") }),
        )));
    };
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: reference,
        character: FactDeclaration::Unknown(text("Not stated")),
        medium: FactDeclaration::Unknown(text("Not stated")),
        context: FactDeclaration::Unknown(text("Not stated")),
        outcome: FactDeclaration::Unknown(text("Not stated")),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Not stated")),
        actual_receiver: FactDeclaration::Unknown(text("Not stated")),
        representation: FactRepresentation::NotRecorded(text("Not stated")),
        summary: text("Notification declaration"),
        provenance: FactProvenance::OperatorNote { note: text("Note") },
    })
    .unwrap();
    FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(values)))
}
pub fn values(provenance: FactProvenance) -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Other(label("Declared classification"))),
        subtype: Some(label("Declared subtype")),
        issuer: FactDeclaration::Known(label("Declared issuer")),
        issued_at: DeclaredProceduralTime::minute("2026-09-16".parse().unwrap(), 13, 24, None)
            .unwrap(),
        summary: text("Captured resolution summary"),
        provenance,
    })
}
pub fn support() -> StageSupportSnapshot {
    StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(91)),
            version: DocumentVersion::new(2).unwrap(),
        },
        digest: digest(9),
        name: "historical-resolution.pdf".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    }
}
pub struct Fixture {
    pub material: ResolutionSourceMaterial,
    pub sources: FactSources,
}
impl Fixture {
    pub fn new(
        revision: u32,
        withdrawn: bool,
        hearing: Option<HearingResultSnapshot>,
        admitted_support: Option<StageSupportSnapshot>,
    ) -> Self {
        assert!(!withdrawn || revision > 1);
        let evidence = admitted_support
            .as_ref()
            .map(|value| FactEvidence::new(value.reference, value.digest, label("Declared page")));
        let mut sources = FactSources {
            resolved: FactResolvedSources {
                resolution: None,
                participants: vec![],
                hearing_results: vec![],
            },
            views: FactSourceViews {
                resolution: None,
                participants: vec![],
                hearing_results: vec![],
            },
            direct_supports: admitted_support.iter().cloned().collect(),
        };
        let provenance = match &hearing {
            Some(value) => {
                let agreement = value.values.agreements().first().cloned();
                let reference = FactHearingRef {
                    hearing_id: value.hearing_id,
                    result_id: value.id,
                    revision: value.revision,
                    agreement_id: agreement.as_ref().map(|value| value.id()),
                };
                sources
                    .resolved
                    .hearing_results
                    .push(FactHearingSourceSnapshot {
                        case_id: value.case_id,
                        reference,
                        values_digest: value.values_digest,
                        submission_digest: value.receipt.submission_digest,
                        status: value.status,
                    });
                sources.views.hearing_results.push(FactHearingView {
                    reference,
                    occurrence: value.values.occurrence(),
                    event_time: value.values.event_time(),
                    summary: value.values.summary().clone(),
                    agreement,
                });
                FactProvenance::HearingResult {
                    reference,
                    locator: label("Declared agreement"),
                    support: evidence,
                }
            }
            None if evidence.is_some() => FactProvenance::ExternalReference {
                reference: text("Historical external record"),
                support: evidence,
            },
            None => FactProvenance::OperatorNote { note: text("Note") },
        };
        let selected = reference(revision);
        let snapshot = ResolutionSnapshot {
            root: ResolutionRoot::new(selected.id, case_id()),
            values: values(provenance),
            metadata: FactRevisionMetadata {
                revision: selected.revision,
                values_digest: digest(49),
                status: if withdrawn {
                    FactStatus::Withdrawn
                } else {
                    FactStatus::Recorded
                },
                reason: (revision > 1).then(|| text("Explicit historical reason")),
                receipt: FactReceipt {
                    operation_id: FactOperationId::from_uuid(Uuid::from_u128(43)),
                    action: if withdrawn {
                        FactAction::Withdraw
                    } else if revision > 1 {
                        FactAction::Correct
                    } else {
                        FactAction::Record
                    },
                    expected_revision: revision - 1,
                    sources_digest: digest(50),
                    submission_digest: digest(51),
                },
                recorded_administration: CurrentCaseAdministration::Unrevised(
                    CaseMetadata::new("Historical case", "CASE-1").unwrap(),
                ),
                recorded_at: OffsetDateTime::UNIX_EPOCH,
                recorded_by: CaseActorSnapshot {
                    id: UserId::from_uuid(Uuid::from_u128(44)),
                    email: "captured@example.test".into(),
                },
            },
        };
        Self {
            material: ResolutionSourceMaterial {
                snapshot,
                hearing,
                admitted_support,
            },
            sources,
        }
    }
    pub fn hasher(&self) -> Hasher {
        let snapshot = &self.material.snapshot;
        let metadata = &snapshot.metadata;
        let mut hasher = Hasher::default();
        hasher
            .expected
            .push((snapshot.values.canonical_bytes(), metadata.values_digest));
        hasher.expected.push((
            fact_sources_bytes(&self.sources).unwrap(),
            metadata.receipt.sources_digest,
        ));
        let change = match metadata.receipt.action {
            FactAction::Record => FactChange::record(snapshot.values.clone()),
            FactAction::Correct => FactChange::correct(
                FactRevision::new(metadata.receipt.expected_revision).unwrap(),
                snapshot.values.clone(),
                metadata.reason.clone().unwrap(),
            ),
            FactAction::Withdraw => FactChange::withdraw(
                FactRevision::new(metadata.receipt.expected_revision).unwrap(),
                metadata.reason.clone().unwrap(),
            ),
        };
        let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
            metadata.receipt.operation_id,
            snapshot.root.id(),
            change,
        ));
        hasher.expected.push((
            fact_submission_bytes(
                metadata.recorded_by.id,
                snapshot.root.case_id(),
                &command,
                metadata.values_digest,
                metadata.receipt.sources_digest,
            )
            .unwrap(),
            metadata.receipt.submission_digest,
        ));
        if let Some(hearing) = &self.material.hearing {
            hasher
                .expected
                .push((hearing.values.canonical_bytes(), hearing.values_digest));
            hasher.expected.push((
                crate::hearing_support::receipt_bytes(hearing),
                hearing.receipt.submission_digest,
            ));
        }
        hasher
    }
}
/// Recognizes canonical fixture bytes without introducing a hash algorithm.
#[derive(Default)]
pub struct Hasher {
    expected: Vec<(Vec<u8>, Sha256Digest)>,
    pub seen: RefCell<Vec<Vec<u8>>>,
}
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.seen.borrow_mut().push(bytes.to_vec());
        self.expected
            .iter()
            .find(|(value, _)| value == bytes)
            .map(|(_, digest)| *digest)
            .unwrap_or_else(|| digest(255))
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("historical resolution verification cannot read document bytes")
    }
}
pub fn assert_inconsistent(result: Result<Option<FactResolutionProjection>, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}

pub fn assert_rejected(hasher: &dyn DocumentHasher, material: &ResolutionSourceMaterial) {
    assert_inconsistent(resolve_fact_resolution(
        hasher,
        case_id(),
        &selection(Some(reference(1))),
        Some(material),
    ));
}
