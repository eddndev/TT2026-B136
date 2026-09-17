use application::{
    cases::CaseActorSnapshot, hearing_results::*, procedural_facts::*, ApplicationError,
};
use domain::{
    case_administration::CaseRevision,
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    hearings::{HearingId, HearingRevision},
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use std::{cell::RefCell, io::Read};
use uuid::Uuid;

/// Recognizes fixture bytes without implementing a digest algorithm.
#[derive(Default)]
pub struct Hasher {
    expected: Vec<(Vec<u8>, Sha256Digest)>,
    pub seen: RefCell<Vec<Vec<u8>>>,
}
impl Hasher {
    pub fn new(snapshots: &[&HearingResultSnapshot]) -> Self {
        let mut value = Self::default();
        for snapshot in snapshots {
            value
                .expected
                .push((snapshot.values.canonical_bytes(), snapshot.values_digest));
            value
                .expected
                .push((receipt_bytes(snapshot), snapshot.receipt.submission_digest));
        }
        value
    }
}
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        self.seen.borrow_mut().push(data.to_vec());
        self.expected
            .iter()
            .find(|(bytes, _)| bytes == data)
            .map(|(_, digest)| *digest)
            .unwrap_or_else(|| Sha256Digest::from_array([255; 32]))
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("historical source validation must use canonical bytes")
    }
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn reference(
    hearing: u128,
    result: u128,
    revision: u32,
    agreement: Option<u128>,
) -> FactHearingRef {
    FactHearingRef {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(hearing)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(result)),
        revision: HearingResultRevision::new(revision).unwrap(),
        agreement_id: agreement.map(|id| HearingResultAgreementId::from_uuid(Uuid::from_u128(id))),
    }
}
fn fact_text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
fn provenance(reference: FactHearingRef) -> FactProvenance {
    FactProvenance::HearingResult {
        reference,
        locator: FactLabel::new("Declared location").unwrap(),
        support: None,
    }
}
fn unlinked() -> FactPerson {
    FactPerson::Unlinked {
        label: FactLabel::new("Declared person").unwrap(),
        description: fact_text("Unlinked declaration"),
    }
}
pub fn selection(references: &[FactHearingRef]) -> FactSourceSelection {
    assert!(references.len() <= 2);
    let primary = references
        .first()
        .copied()
        .map(provenance)
        .unwrap_or_else(|| FactProvenance::OperatorNote {
            note: fact_text("Operator note"),
        });
    let representation = references
        .get(1)
        .copied()
        .map(|reference| FactRepresentation::Declared {
            represented: unlinked(),
            representative: unlinked(),
            scope: fact_text("Declared relation"),
            provenance: Box::new(provenance(reference)),
        })
        .unwrap_or_else(|| FactRepresentation::NotRecorded(fact_text("Not stated")));
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(3)),
            revision: FactRevision::initial(),
        },
        character: FactDeclaration::Unknown(fact_text("Not stated")),
        medium: FactDeclaration::Unknown(fact_text("Not stated")),
        context: FactDeclaration::Unknown(fact_text("Not stated")),
        outcome: FactDeclaration::Unknown(fact_text("Not stated")),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(fact_text("Not stated")),
        actual_receiver: FactDeclaration::Unknown(fact_text("Not stated")),
        representation,
        summary: fact_text("Declared notification"),
        provenance: primary,
    })
    .unwrap();
    FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(values)))
}
pub fn values_input(values: &HearingResultValues) -> HearingResultValuesInput {
    HearingResultValuesInput {
        occurrence: values.occurrence(),
        extent: values.extent(),
        event_time: values.event_time(),
        summary: values.summary().clone(),
        attendees: values.attendees().to_vec(),
        agreements: values.agreements().to_vec(),
        provenance: values.provenance().clone(),
    }
}
pub fn source(
    hearing: u128,
    result: u128,
    revision: u32,
    withdrawn: bool,
    agreements: &[u128],
) -> HearingResultSnapshot {
    assert!(!withdrawn || revision > 1);
    let selected = reference(hearing, result, revision, None);
    let values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Partial,
        event_time: DeclaredHearingResultTime::date(
            OffsetDateTime::UNIX_EPOCH.date(),
            time::UtcOffset::UTC,
        )
        .unwrap(),
        summary: HearingResultText::new(&format!("Result {result} revision {revision}")).unwrap(),
        attendees: vec![],
        agreements: agreements
            .iter()
            .map(|id| {
                HearingResultAgreement::new(
                    HearingResultAgreementId::from_uuid(Uuid::from_u128(*id)),
                    HearingResultText::new(&format!("Agreement {id} in revision {revision}"))
                        .unwrap(),
                )
            })
            .collect(),
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::WrittenRecord,
            Some(HearingResultReference::new("Historical minutes").unwrap()),
            Some(HearingResultSupportRef::new(
                DocumentVersionRef {
                    id: DocumentId::from_uuid(Uuid::from_u128(90)),
                    version: DocumentVersion::initial(),
                },
                Sha256Digest::from_array([9; 32]),
            )),
        )
        .unwrap(),
    })
    .unwrap();
    HearingResultSnapshot {
        case_id: case_id(),
        hearing_id: selected.hearing_id,
        id: selected.result_id,
        revision: selected.revision,
        values,
        values_digest: Sha256Digest::from_array([17; 32]),
        status: if withdrawn {
            HearingResultStatus::Withdrawn
        } else {
            HearingResultStatus::Recorded
        },
        reason: (revision > 1).then(|| HearingResultText::new("Explicit revision reason").unwrap()),
        receipt: HearingResultReceipt {
            operation_id: HearingResultOperationId::from_uuid(Uuid::from_u128(result + 100)),
            action: if withdrawn {
                HearingResultAction::Withdraw
            } else if revision == 1 {
                HearingResultAction::Record
            } else {
                HearingResultAction::Correct
            },
            expected_revision: revision - 1,
            submission_digest: Sha256Digest::from_array([34; 32]),
        },
        anchor: HearingResultAnchor {
            hearing_id: selected.hearing_id,
            revision: HearingRevision::initial(),
            values_digest: Sha256Digest::from_array([5; 32]),
            submission_digest: Sha256Digest::from_array([6; 32]),
        },
        continuation: None,
        recorded_administration_revision: CaseRevision::FIRST,
        recorded_administration_digest: Sha256Digest::from_array([7; 32]),
        recorded_at: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(8)),
            email: "captured@example.test".into(),
        },
    }
}
pub fn receipt_bytes(snapshot: &HearingResultSnapshot) -> Vec<u8> {
    let change = match snapshot.receipt.action {
        HearingResultAction::Record => HearingResultChange::Record {
            anchor_revision: snapshot.anchor.revision,
            continuation: snapshot
                .continuation
                .map(|source| HearingResultContinuationRef::new(source.result_id, source.revision)),
            values: snapshot.values.clone(),
        },
        HearingResultAction::Correct => HearingResultChange::Correct {
            expected_revision: HearingResultRevision::new(snapshot.receipt.expected_revision)
                .unwrap(),
            values: snapshot.values.clone(),
            reason: snapshot.reason.clone().unwrap(),
        },
        HearingResultAction::Withdraw => HearingResultChange::Withdraw {
            expected_revision: HearingResultRevision::new(snapshot.receipt.expected_revision)
                .unwrap(),
            reason: snapshot.reason.clone().unwrap(),
        },
    };
    let command = HearingResultCommand {
        operation_id: snapshot.receipt.operation_id,
        hearing_id: snapshot.hearing_id,
        result_id: snapshot.id,
        change,
    };
    hearing_result_submission_bytes(
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command,
        &snapshot.anchor,
        snapshot.continuation.as_ref(),
        snapshot.values_digest,
    )
}
pub fn assert_inconsistent(result: Result<Vec<FactHearingProjection>, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}
