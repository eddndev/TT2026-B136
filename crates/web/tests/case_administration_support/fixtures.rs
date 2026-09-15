use application::cases::*;
use domain::{cases::CaseMetadata, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

pub const CASE: &str = "00000000-0000-0000-0000-000000000001";
pub const ACTOR: &str = "00000000-0000-0000-0000-000000000003";

pub fn item() -> String {
    format!("/api/v1/cases/{CASE}/administration")
}
pub fn status_path() -> String {
    format!("/api/v1/cases/{CASE}/administrative-status")
}
pub fn actor(email: &str) -> CaseActorSnapshot {
    CaseActorSnapshot {
        id: UserId::from_uuid(Uuid::from_u128(3)),
        email: email.into(),
    }
}
pub fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_700_000_000)
        .unwrap()
        .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap())
}
pub fn origin() -> CaseOrigin {
    CaseOrigin {
        id: domain::cases::CaseId::from_uuid(Uuid::from_u128(1)),
        created_by: UserId::from_uuid(Uuid::from_u128(2)),
        created_at: at(),
    }
}
pub fn metadata() -> CaseMetadata {
    CaseMetadata::new("Current title", "REF-001").unwrap()
}
pub fn profile() -> PenalCaseProfile {
    PenalCaseProfile::new(
        "NUC-001",
        "Recorded authority",
        "CJ-001",
        "Recorded court",
        &["Offense A", "Offense B"],
        Some("Line one\nLine two"),
        None,
    )
    .unwrap()
}
pub fn snapshot() -> CaseAdministrationSnapshot {
    CaseAdministrationSnapshot {
        case_id: origin().id,
        revision: CaseRevision::new(3).unwrap(),
        values: CaseAdministrationValues::new(
            CaseEditableValues::new(metadata(), Some(profile())),
            CaseAdministrativeStatus::Active,
        ),
        values_digest: Sha256Digest::from_bytes(&[0x33; 32]).unwrap(),
        changed_at: at(),
        changed_by: actor("captured@example.com"),
    }
}
pub fn detail() -> CaseAdministrationDetail {
    CaseAdministrationDetail {
        origin: origin(),
        administration: CurrentCaseAdministration::Recorded(Box::new(snapshot())),
        initial_stage: Some(CaseInitialStageRegistration {
            case_id: origin().id,
            stage_revision: CaseStageRevision::FIRST,
            administration_revision: CaseRevision::FIRST,
            stage: InitialCaseStage::Investigation,
            administration_digest: Sha256Digest::from_bytes(&[0x11; 32]).unwrap(),
            recorded_at: at(),
            recorded_by: actor("initial@example.com"),
        }),
    }
}
pub fn creation() -> Value {
    json!({"title":" Current title ","reference":" REF-001 ","profile":{
        "nuc":" NUC-001 ","nuc_authority":"Recorded authority",
        "judicial_case_number":"CJ-001","judicial_authority":"Recorded court",
        "offenses":["Offense A","Offense B"],
        "general_information":" Line one\r\nLine two ",
        "complementary_identifiers":" "}})
}
pub fn replacement(expected: u32) -> Value {
    let mut input = creation();
    input["expected_revision"] = json!(expected);
    input
}
