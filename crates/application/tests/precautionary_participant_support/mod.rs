use application::participants::{
    participant_digest, ParticipantActorSnapshot, ParticipantSnapshot,
};
use application::precautionary_hearings::resolve_precautionary_participants;
use application::typed_participants::*;
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::{
    DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::hearings::{
    HearingModality, HearingNote, HearingParticipantRef, HearingSupportRef, HearingTime,
    HearingVenue,
};
use domain::identity::UserId;
use domain::participants::{
    DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantValues,
};
use domain::precautionary_hearings::{
    PrecautionaryHearingPurpose, PrecautionaryHearingSchedulingBasis, PrecautionaryHearingValues,
    PrecautionaryHearingValuesInput,
};
use domain::DomainError;
use std::io::Read;
use uuid::Uuid;

pub struct Hasher;

impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut result = [0_u8; 32];
        for (index, byte) in data.iter().enumerate() {
            result[index % 32] = result[index % 32]
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
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

pub fn reference(id: u128, revision: u32) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
    )
}

fn support() -> HearingSupportRef {
    HearingSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(90)),
            version: DocumentVersion::initial(),
        },
        Sha256Digest::from_array([9; 32]),
    )
}

pub fn hearing_input(references: &[(u128, u32)]) -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Imposition,
        scheduled_at: HearingTime::new(OffsetDateTime::UNIX_EPOCH).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: None,
        participants: references
            .iter()
            .map(|(id, rev)| reference(*id, *rev))
            .collect(),
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Declared appointment").unwrap(),
            support(),
            HearingNote::new("Page 1").unwrap(),
        ),
        review_targets: Vec::new(),
    }
}

pub fn values(references: &[(u128, u32)]) -> PrecautionaryHearingValues {
    PrecautionaryHearingValues::new(hearing_input(references)).unwrap()
}

pub fn assert_rejected(references: &[(u128, u32)], material: &[ParticipantDetail]) {
    assert!(
        resolve_precautionary_participants(&Hasher, case_id(), &values(references), material,)
            .is_err()
    );
}

fn actor() -> ParticipantActorSnapshot {
    ParticipantActorSnapshot {
        id: UserId::from_uuid(Uuid::from_u128(4)),
        email: "author@example.test".into(),
    }
}

pub fn manual(id: u128, revision: u32, status: DirectoryStatus) -> ParticipantDetail {
    let values = ParticipantValues::new(
        "Historical manual name",
        "Declared role",
        Some("Organization"),
        Some("Private legal detail"),
        status,
    )
    .unwrap();
    ParticipantSnapshot {
        case_id: case_id(),
        id: reference(id, revision).id(),
        revision: reference(id, revision).revision(),
        values_digest: participant_digest(&Hasher, &values),
        values,
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: actor(),
    }
    .into()
}

fn evidence() -> ParticipantEvidenceLocator {
    ParticipantEvidenceLocator::new(support().reference(), support().digest(), "Page 1").unwrap()
}

pub fn subject_values(institutional: bool) -> SubjectValues {
    if institutional {
        SubjectValues::institutional_body(
            ParticipantText::new("Historical court").unwrap(),
            Declared::Known(ParticipantText::new("Declared identifier").unwrap()),
            evidence(),
        )
    } else {
        SubjectValues::natural_person(
            RepresentedName::Known(ParticipantText::new("Historical person").unwrap()),
            Declared::Unknown(ParticipantReason::new("Identifier not stated").unwrap()),
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
            id: reference(id, revision).id(),
            revision: reference(id, revision).revision(),
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
