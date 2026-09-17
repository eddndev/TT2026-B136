#![allow(dead_code)]
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    participants::{ParticipantId, ParticipantRevision},
    procedural_facts::*,
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn person(id: u128, revision: u32) -> FactPerson {
    FactPerson::Participant(FactParticipantRef {
        id: ParticipantId::from_uuid(Uuid::from_u128(id)),
        revision: ParticipantRevision::new(revision).unwrap(),
    })
}
pub fn evidence(id: u128, version: u32, byte: u8, locator: &str) -> FactEvidence {
    FactEvidence::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(id)),
            version: DocumentVersion::new(version).unwrap(),
        },
        Sha256Digest::from_array([byte; 32]),
        label(locator),
    )
}
pub fn external(support: Option<FactEvidence>) -> FactProvenance {
    FactProvenance::ExternalReference {
        reference: text("Declared source"),
        support,
    }
}
pub fn resolution_input() -> ResolutionValuesInput {
    ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Issuer not stated")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text("Resolution declared by operator"),
        provenance: FactProvenance::OperatorNote {
            note: text("Intake note"),
        },
    }
}
pub fn notification_input() -> NotificationValuesInput {
    NotificationValuesInput {
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(100)),
            revision: FactRevision::initial(),
        },
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::Electronic),
        context: FactDeclaration::Unknown(text("Context not stated")),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Known(person(1, 2)),
        actual_receiver: FactDeclaration::Unknown(text("Receiver not stated")),
        representation: FactRepresentation::NotRecorded(text("No relationship captured")),
        summary: text("Notification practice declared by operator"),
        provenance: FactProvenance::OperatorNote {
            note: text("Intake note"),
        },
    }
}
