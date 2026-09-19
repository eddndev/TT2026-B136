#![allow(dead_code)]
use crate::case_administration_support::Fixture;
use application::deadline_reevaluation::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn entry(role: ObservationRole, family: DependencyFamily, id: u128) -> ObservationEntry {
    ObservationEntry {
        role,
        family,
        id: Uuid::from_u128(id),
        revision: 3,
        case_id: None,
        hearing_id: None,
        parent_resolution: None,
        submission_digest: Sha256Digest::from_array([0x11; 32]),
        evidence_digest: Sha256Digest::from_array([0x22; 32]),
    }
}
pub fn profile() -> Observations {
    Observations {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        entries: vec![entry(
            ObservationRole::Profile,
            DependencyFamily::Profile,
            0xabcd,
        )],
    }
}
pub fn notification() -> Observations {
    let mut value = profile();
    value.entries[0].case_id = Some(value.case_id);
    let mut source = entry(ObservationRole::Source, DependencyFamily::Notification, 3);
    source.case_id = Some(value.case_id);
    source.parent_resolution = Some(ResolutionReference {
        id: Uuid::from_u128(4),
        revision: 1,
    });
    value.entries.push(source);
    value.entries.push(entry(
        ObservationRole::Calendar,
        DependencyFamily::Calendar,
        5,
    ));
    let mut parent = entry(
        ObservationRole::NotificationParent,
        DependencyFamily::Resolution,
        4,
    );
    parent.case_id = Some(value.case_id);
    parent.revision = 9;
    value.entries.push(parent);
    value
}

/// Independent framing also represents invalid combinations rejected by the encoder.
pub fn wire(value: &Observations) -> Vec<u8> {
    let mut bytes = b"DLOB1".to_vec();
    bytes.extend(value.case_id.as_uuid().as_bytes());
    bytes.push(value.entries.len() as u8);
    for entry in &value.entries {
        bytes.extend([entry.role as u8, entry.family as u8]);
        bytes.extend(entry.id.as_bytes());
        bytes.extend(entry.revision.to_be_bytes());
        bytes.push(u8::from(entry.case_id.is_some()));
        if let Some(case) = entry.case_id {
            bytes.extend(case.as_uuid().as_bytes());
        }
        bytes.push(u8::from(entry.hearing_id.is_some()));
        if let Some(hearing) = entry.hearing_id {
            bytes.extend(hearing.as_bytes());
        }
        bytes.push(u8::from(entry.parent_resolution.is_some()));
        if let Some(parent) = entry.parent_resolution {
            bytes.extend(parent.id.as_bytes());
            bytes.extend(parent.revision.to_be_bytes());
        }
        bytes.extend(entry.submission_digest.as_bytes());
        bytes.extend(entry.evidence_digest.as_bytes());
    }
    bytes
}
pub fn expected(value: &Observations) -> Value {
    let entries: Vec<_> = value.entries.iter().map(|entry| json!({
        "role": match entry.role {
            ObservationRole::Profile => "profile", ObservationRole::Source => "source",
            ObservationRole::Calendar => "calendar",
            ObservationRole::NotificationParent => "notification_parent",
        },
        "family": match entry.family {
            DependencyFamily::Resolution => "resolution",
            DependencyFamily::Notification => "notification",
            DependencyFamily::HearingResult => "hearing_result",
            DependencyFamily::Calendar => "calendar", DependencyFamily::Profile => "profile",
        },
        "id": entry.id.to_string(), "revision": entry.revision,
        "case_id": entry.case_id.map(|id| id.to_string()),
        "hearing_id": entry.hearing_id.map(|id| id.to_string()),
        "parent_resolution": entry.parent_resolution.map(|parent| json!({
            "id": parent.id.to_string(), "revision": parent.revision,
        })),
        "submission_digest": entry.submission_digest.to_hex(),
        "evidence_digest": entry.evidence_digest.to_hex(),
    })).collect();
    json!({"case_id": value.case_id.to_string(), "entries": entries})
}
pub fn parse(db: &mut Fixture, bytes: &[u8]) -> Value {
    db.admin
        .query_one("SELECT deadline_observations($1::bytea)", &[&bytes])
        .unwrap()
        .get(0)
}
pub fn fixture() -> Option<Fixture> {
    let mut db = Fixture::new()?;
    // A missing parser must fail negatives instead of being mistaken for rejection.
    parity(&mut db, &profile());
    Some(db)
}
pub fn parity(db: &mut Fixture, value: &Observations) -> Vec<u8> {
    let bytes = encode_observations(value).unwrap();
    assert_eq!(bytes, wire(value));
    assert_eq!(decode_observations(&bytes).unwrap(), *value);
    assert_eq!(parse(db, &bytes), expected(value));
    bytes
}
pub fn rejected(db: &mut Fixture, bytes: &[u8]) {
    assert!(
        decode_observations(bytes).is_err(),
        "invalid fixture decoded in Rust"
    );
    let error = db
        .admin
        .query_one("SELECT deadline_observations($1::bytea)", &[&bytes])
        .expect_err("PostgreSQL accepted invalid DLOB1");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
}
