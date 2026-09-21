use super::*;
use application::{procedural_resources::*, resource_activities::*};
use domain::{
    procedural_facts::{FactDeclaration, FactText},
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn reference() -> ResourceActCaptureRef {
    ResourceActCaptureRef {
        id: ResourceActId::from_uuid(ACT.parse().unwrap()),
        revision: ResourceActRevision::initial(),
        resource_revision: ResourceRevision::new(3).unwrap(),
        capture_digest: digest(),
    }
}

pub fn source() -> ResourceDetail {
    let unknown = FactText::new("Not declared in exact support").unwrap();
    let values = ResourceActValues::new(ResourceActValuesInput {
        kind: ResourceActKind::Interposition,
        mode: FactDeclaration::Known(ResourceMode::Oral),
        occurred_at: DeclaredProceduralTime::unknown(),
        authority: FactDeclaration::Unknown(unknown),
        statement: FactText::new("Declared oral act").unwrap(),
        evidence: vec![resources::values().resolution_evidence().clone()],
    })
    .unwrap();
    resources::detail(
        case(),
        &ResourceCommand {
            operation_id: ResourceOperationId::from_uuid(Uuid::from_u128(8)),
            resource_id: resource(),
            change: ResourceChange::RecordAct {
                expected_revision: ResourceRevision::new(2).unwrap(),
                act_id: reference().id,
                values,
            },
        },
    )
}
