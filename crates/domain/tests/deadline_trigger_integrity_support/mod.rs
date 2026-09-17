#![allow(dead_code)]
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    deadline_triggers::*,
    hearing_results::{HearingResultId, HearingResultRevision, HearingResultValues},
    hearings::HearingId,
    procedural_facts::*,
};
use uuid::Uuid;

pub fn case(id: u128) -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(id))
}
pub fn resolution() -> FactResolutionRef {
    FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(100)),
        revision: FactRevision::new(3).unwrap(),
    }
}
pub fn notification_id() -> NotificationId {
    NotificationId::from_uuid(Uuid::from_u128(200))
}
pub fn hearing() -> FactHearingRef {
    FactHearingRef {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(300)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(400)),
        revision: HearingResultRevision::new(5).unwrap(),
        agreement_id: None,
    }
}
pub fn notice_ref() -> TriggerSourceRef {
    TriggerSourceRef::Notification {
        id: notification_id(),
        revision: FactRevision::new(4).unwrap(),
        resolution: resolution(),
    }
}
pub fn selection(source: TriggerSourceRef) -> TriggerSelection {
    TriggerSelection {
        case_id: case(1),
        source: FactDeclaration::Known(source),
        qualification: None,
    }
}
pub fn fact_digests() -> FactTriggerDigests {
    FactTriggerDigests {
        values: Sha256Digest::from_array([0x11; 32]),
        sources: Sha256Digest::from_array([0x22; 32]),
        submission: Sha256Digest::from_array([0x33; 32]),
    }
}
pub fn hearing_digests() -> HearingTriggerDigests {
    HearingTriggerDigests {
        values: Sha256Digest::from_array([0x44; 32]),
        submission: Sha256Digest::from_array([0x55; 32]),
    }
}
pub fn resolution_material(values: &ResolutionValues) -> TriggerMaterial<'_> {
    TriggerMaterial::Resolution {
        root: ResolutionRoot::new(resolution().id, case(1)),
        revision: resolution().revision,
        values,
        digests: fact_digests(),
    }
}
pub fn notification_material(values: &NotificationValues) -> TriggerMaterial<'_> {
    TriggerMaterial::Notification {
        root: NotificationRoot::new(notification_id(), case(1), resolution().id),
        revision: FactRevision::new(4).unwrap(),
        values,
        digests: fact_digests(),
    }
}
pub fn hearing_material(values: &HearingResultValues) -> TriggerMaterial<'_> {
    let reference = hearing();
    TriggerMaterial::HearingResult {
        case_id: case(1),
        hearing_id: reference.hearing_id,
        result_id: reference.result_id,
        revision: reference.revision,
        values,
        digests: hearing_digests(),
    }
}
pub fn reject(
    field: TriggerField,
    selected: &TriggerSelection,
    material: Option<TriggerMaterial<'_>>,
    expected: TriggerIntegrityError,
) {
    let error = extract_trigger_time(TriggerRequirement::SourceField(field), selected, material)
        .unwrap_err();
    assert_eq!(error, expected);
}
