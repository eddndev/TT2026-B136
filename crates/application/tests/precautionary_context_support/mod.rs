use std::io::Read;

use application::case_stages::*;
use application::cases::*;
use application::precautionary_hearings::{PrecautionaryContext, PrecautionaryContextMaterial};
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::{
    DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::identity::UserId;
use domain::DomainError;
use time::{Duration, OffsetDateTime};

pub struct Hasher;

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

pub fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1_800_000_000_123_456_789).unwrap()
}

pub fn actor() -> CaseActorSnapshot {
    CaseActorSnapshot {
        id: UserId::from_uuid(uuid::Uuid::from_u128(2)),
        email: "captured@example.com".into(),
    }
}

pub fn administration_values() -> CaseAdministrationValues {
    PenalCaseCreation::new(
        CaseMetadata::new("Penal title", "REF-1").unwrap(),
        PenalCaseProfile::new(
            "NUC-1",
            "Prosecution",
            "CJ-1",
            "Court",
            &["Offense A", "Offense B"],
            Some("General information"),
            Some("Complementary identifiers"),
        )
        .unwrap(),
    )
    .into_values()
}

pub fn initial() -> PrecautionaryContextMaterial {
    let case_id = CaseId::from_uuid(uuid::Uuid::from_u128(1));
    let values = administration_values();
    let administration = CaseAdministrationSnapshot {
        case_id,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(&Hasher, &values),
        values,
        changed_at: at(),
        changed_by: actor(),
    };
    PrecautionaryContextMaterial {
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
    }
}

pub fn support(index: u128) -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid::Uuid::from_u128(index)),
            version: DocumentVersion::new(3).unwrap(),
        },
        Sha256Digest::from_array([index as u8; 32]),
    )
}

pub fn adoption(stage: CaseStage) -> CaseStageChange {
    CaseStageChange::Adopt(StageAdoption::new(
        stage,
        DeclaredStageTime::instant(at()).unwrap(),
        StageNote::new("Known documentary history").unwrap(),
        support(3),
    ))
}

pub fn intermediate() -> CaseStageChange {
    CaseStageChange::Transition(StageTransition::to_intermediate(
        DeclaredStageTime::instant(at()).unwrap(),
        support(3),
        Some(StageNote::new("Declared accusation").unwrap()),
    ))
}

pub fn trial() -> CaseStageChange {
    CaseStageChange::Transition(
        StageTransition::to_trial(
            DeclaredStageTime::instant(at()).unwrap(),
            support(3),
            DeclaredStageTime::instant(at() + Duration::seconds(1)).unwrap(),
            StageCourt::new("Trial court").unwrap(),
            Some(StageReceiptReference::new("RECEIPT-1").unwrap()),
            Some(support(4)),
            Some(StageNote::new("Declared receipt").unwrap()),
        )
        .unwrap(),
    )
}

pub fn changed(values: CaseStageChange) -> PrecautionaryContextMaterial {
    let mut material = initial();
    let (revision, from_stage) = match &values {
        CaseStageChange::Adopt(_) => (1, None),
        CaseStageChange::Transition(StageTransition::ToIntermediate(_)) => {
            (2, Some(CaseStage::Investigation))
        }
        CaseStageChange::Transition(StageTransition::ToTrial(_)) => {
            (3, Some(CaseStage::Intermediate))
        }
    };
    let supports = values
        .supports()
        .into_iter()
        .map(|selected| StageSupportSnapshot {
            reference: selected.reference(),
            digest: selected.digest(),
            name: "retained-support.pdf".into(),
            format: StageDocumentFormat::Pdf,
            policy: StageFormatPolicy::PdfDocxV1,
        })
        .collect();
    material.stage = CaseStageEntry::Changed(Box::new(CaseStageSnapshot {
        case_id: material.case_id,
        stage_revision: CaseStageRevision::new(revision).unwrap(),
        from_stage,
        values_digest: case_stage_digest(&Hasher, &values),
        values,
        administration_revision: material.stage_administration.revision,
        administration_digest: material.stage_administration.values_digest,
        supports,
        recorded_at: at() + Duration::seconds(10),
        recorded_by: actor(),
    }));
    material
}

pub fn changed_mut(material: &mut PrecautionaryContextMaterial) -> &mut CaseStageSnapshot {
    match &mut material.stage {
        CaseStageEntry::Changed(stage) => stage,
        CaseStageEntry::Initial(_) => panic!("expected changed stage fixture"),
    }
}

pub fn observed_newer(material: &mut PrecautionaryContextMaterial) {
    material.administration.revision = CaseRevision::new(7).unwrap();
    material.administration.changed_at = at() + Duration::seconds(20);
    material.administration.changed_by.email = "later@example.com".into();
}

pub fn rejected(material: PrecautionaryContextMaterial) {
    assert!(PrecautionaryContext::new(&Hasher, material).is_err());
}

pub fn bytes(material: PrecautionaryContextMaterial) -> Vec<u8> {
    PrecautionaryContext::new(&Hasher, material)
        .unwrap()
        .canonical_bytes()
}
