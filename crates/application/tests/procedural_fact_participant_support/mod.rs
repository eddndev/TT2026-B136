use application::{
    participants::{participant_digest, ParticipantActorSnapshot, ParticipantSnapshot},
    procedural_facts::*,
    typed_participants::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::*,
    identity::UserId,
    participants::{DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantValues},
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use std::io::Read;
use uuid::Uuid;

pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut result = [0u8; 32];
        for (i, byte) in data.iter().enumerate() {
            result[i % 32] = result[i % 32].wrapping_add(*byte).wrapping_add(i as u8);
        }
        Sha256Digest::from_array(result)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        Ok(self.hash_bytes(&bytes))
    }
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn other_case() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(2))
}
fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn reference(id: u128, revision: u32) -> FactParticipantRef {
    FactParticipantRef {
        id: ParticipantId::from_uuid(Uuid::from_u128(id)),
        revision: ParticipantRevision::new(revision).unwrap(),
    }
}
pub fn selection(references: &[(u128, u32)]) -> FactSourceSelection {
    assert!(references.len() <= 4);
    let person = |index: usize| {
        references
            .get(index)
            .map(|(id, revision)| FactPerson::Participant(reference(*id, *revision)))
    };
    let declaration = |index| match person(index) {
        Some(value) => FactDeclaration::Known(value),
        None => FactDeclaration::Unknown(text("Not stated")),
    };
    let representation = match (person(2), person(3)) {
        (Some(represented), Some(representative)) => FactRepresentation::Declared {
            represented,
            representative,
            scope: text("Declared scope"),
            provenance: Box::new(FactProvenance::OperatorNote {
                note: text("Declared source"),
            }),
        },
        (None, None) => FactRepresentation::NotRecorded(text("Not stated")),
        _ => panic!("fixture requires zero, one, two or four references"),
    };
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(3)),
            revision: FactRevision::initial(),
        },
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::InPerson),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: declaration(0),
        actual_receiver: declaration(1),
        representation,
        summary: text("Declared practice"),
        provenance: FactProvenance::OperatorNote {
            note: text("Intake note"),
        },
    })
    .unwrap();
    FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(values)))
}
fn actor() -> ParticipantActorSnapshot {
    ParticipantActorSnapshot {
        id: UserId::from_uuid(Uuid::from_u128(4)),
        email: "author@example.test".into(),
    }
}
pub fn manual(id: u128, revision: u32, status: DirectoryStatus) -> ParticipantDetail {
    let values = ParticipantValues::new(
        "Manual name",
        "Declared role",
        Some("Organization"),
        Some("Private legal detail"),
        status,
    )
    .unwrap();
    ParticipantSnapshot {
        case_id: case_id(),
        id: reference(id, revision).id,
        revision: reference(id, revision).revision,
        values_digest: participant_digest(&Hasher, &values),
        values,
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: actor(),
    }
    .into()
}
fn evidence() -> ParticipantEvidenceLocator {
    ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(90)),
            version: DocumentVersion::initial(),
        },
        Sha256Digest::from_array([9; 32]),
        "p. 1",
    )
    .unwrap()
}
pub fn subject_values(institutional: bool) -> SubjectValues {
    if institutional {
        SubjectValues::institutional_body(
            ParticipantText::new("Historical court").unwrap(),
            Declared::Known(ParticipantText::new("Private institutional identifier").unwrap()),
            evidence(),
        )
    } else {
        SubjectValues::natural_person(
            RepresentedName::Known(ParticipantText::new("Historical person").unwrap()),
            Declared::Unknown(ParticipantReason::new("Private identity reason").unwrap()),
            evidence(),
        )
    }
}
pub fn typed(
    id: u128,
    revision: u32,
    institutional: bool,
    status: DirectoryStatus,
) -> ParticipantDetail {
    let subject_values = subject_values(institutional);
    let subject = SubjectSnapshot {
        case_id: case_id(),
        id: CaseSubjectId::from_uuid(Uuid::from_u128(50)),
        revision: SubjectRevision::new(7).unwrap(),
        values_digest: subject_digest(&Hasher, &subject_values),
        values: subject_values,
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: actor(),
    };
    let bound = SubjectRevisionRef {
        id: subject.id,
        revision: subject.revision,
        values_digest: subject.values_digest,
    };
    let profile = if institutional {
        ParticipantProfile::TrialCourt(TrialCourtProfile::new(
            ParticipantText::new("Judicial district").unwrap(),
            CourtComposition::Collegiate,
        ))
    } else {
        ParticipantProfile::Defendant(DefendantProfile::new(Declared::Known(
            CustodyState::AtLiberty,
        )))
    };
    let role = ParticipantRoleValues::new(
        Some("Role organization"),
        Some("Private legal status"),
        profile,
        evidence(),
    )
    .unwrap();
    let values = TypedParticipantValues::new(bound, status, role);
    ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(TypedParticipantSnapshot {
            case_id: case_id(),
            id: reference(id, revision).id,
            revision: reference(id, revision).revision,
            values_digest: typed_participant_digest(&Hasher, &values),
            values,
            changed_at: OffsetDateTime::UNIX_EPOCH,
            changed_by: actor(),
            credential_origin: None,
            submission_digest: Sha256Digest::from_array([8; 32]),
            submission_revision: ParticipantRevision::initial(),
        })),
        bound_subject: Some(subject),
    }
}
pub fn manual_mut(detail: &mut ParticipantDetail) -> &mut ParticipantSnapshot {
    let ParticipantRevisionSnapshot::Manual(value) = &mut detail.revision else {
        panic!("manual fixture expected")
    };
    value
}
pub fn typed_mut(detail: &mut ParticipantDetail) -> &mut TypedParticipantSnapshot {
    let ParticipantRevisionSnapshot::Typed(value) = &mut detail.revision else {
        panic!("typed fixture expected")
    };
    value
}
pub fn assert_inconsistent(result: Result<Vec<FactParticipantProjection>, ApplicationError>) {
    let Err(ApplicationError::ProceduralFact(ProceduralFactError::StoredInconsistent(message))) =
        result
    else {
        panic!("expected stored inconsistency, got {result:?}")
    };
    assert!(!message.is_empty());
}
