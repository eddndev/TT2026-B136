#![allow(dead_code)]
use crate::procedural_fact_support::{evidence, label, text};
use domain::{
    participants::{ParticipantId, ParticipantRevision},
    procedural_facts::*,
    procedural_resources::*,
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn appellant(id: Option<u128>, revision: u32, name: &str) -> ResourceAppellant {
    ResourceAppellant::new(
        label(name),
        FactDeclaration::Known(label("Declared affected party")),
        id.map(|id| FactParticipantRef {
            id: ParticipantId::from_uuid(Uuid::from_u128(id)),
            revision: ParticipantRevision::new(revision).unwrap(),
        }),
    )
}
pub fn input() -> ResourceValuesInput {
    ResourceValuesInput {
        kind: ResourceKind::Revocation,
        mode: FactDeclaration::Known(ResourceMode::Written),
        title: label("Written revocation record"),
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(10)),
            revision: FactRevision::new(2).unwrap(),
        },
        resolution_evidence: evidence(20, 3, 42, "Order page 2"),
        resolution_reference: FactDeclaration::Known(label("Declared order reference")),
        issuing_authority: FactDeclaration::Unknown(text("Issuer not recorded")),
        receiving_authority: None,
        resolution_at: DeclaredProceduralTime::unknown(),
        notification_at: None,
        challenged_part: text("Declared challenged paragraph"),
        grounds: text("Reasons stated by the operator"),
        appellants: vec![appellant(Some(30), 4, "Captured name")],
    }
}
pub fn act_input() -> ResourceActValuesInput {
    ResourceActValuesInput {
        kind: ResourceActKind::Interposition,
        mode: FactDeclaration::Known(ResourceMode::Written),
        occurred_at: DeclaredProceduralTime::unknown(),
        authority: FactDeclaration::Unknown(text("Receiving authority not recorded")),
        statement: text("Presentation declared without a legal effectiveness finding"),
        evidence: vec![evidence(40, 2, 21, "Presentation receipt page 1")],
    }
}
