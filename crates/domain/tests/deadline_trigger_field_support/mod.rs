use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    deadline_triggers::{FactTriggerDigests, TriggerMaterial, TriggerSelection, TriggerSourceRef},
    procedural_facts::{
        FactDeclaration, FactResolutionRef, FactRevision, NotificationId, NotificationRoot,
        NotificationValues, NotificationValuesInput, ResolutionId, ResolutionRoot,
        ResolutionValues,
    },
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn parent() -> FactResolutionRef {
    FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(100)),
        revision: FactRevision::new(7).unwrap(),
    }
}
pub fn notification_ref() -> TriggerSourceRef {
    TriggerSourceRef::Notification {
        id: NotificationId::from_uuid(Uuid::from_u128(200)),
        revision: FactRevision::new(9).unwrap(),
        resolution: parent(),
    }
}
pub fn selection(reference: TriggerSourceRef) -> TriggerSelection {
    TriggerSelection {
        case_id: case_id(),
        source: FactDeclaration::Known(reference),
        qualification: None,
    }
}
pub fn digests() -> FactTriggerDigests {
    FactTriggerDigests {
        values: Sha256Digest::from_array([1; 32]),
        sources: Sha256Digest::from_array([2; 32]),
        submission: Sha256Digest::from_array([3; 32]),
    }
}
pub fn resolution_material(values: &ResolutionValues) -> TriggerMaterial<'_> {
    TriggerMaterial::Resolution {
        root: ResolutionRoot::new(parent().id, case_id()),
        revision: parent().revision,
        values,
        digests: digests(),
    }
}
pub fn notification_input() -> NotificationValuesInput {
    let mut input = super::procedural_fact_support::notification_input();
    input.resolution = parent();
    input
}
pub fn notification_material(values: &NotificationValues) -> TriggerMaterial<'_> {
    TriggerMaterial::Notification {
        root: NotificationRoot::new(
            NotificationId::from_uuid(Uuid::from_u128(200)),
            case_id(),
            parent().id,
        ),
        revision: FactRevision::new(9).unwrap(),
        values,
        digests: digests(),
    }
}
pub fn date(value: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(value.parse().unwrap(), None).unwrap()
}
