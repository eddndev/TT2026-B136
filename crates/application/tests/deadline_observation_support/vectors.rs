use super::inputs;
use application::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    deadline_profiles::*,
    judicial_calendars::JudicialCalendarDetail,
    procedural_facts::*,
};
use domain::{crypto::Sha256Digest, identity::UserId};
use time::OffsetDateTime;

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend((value.len() as u64).to_be_bytes());
    bytes.extend(value.as_bytes());
}
fn actor(bytes: &mut Vec<u8>, id: UserId, email: &str, at: OffsetDateTime) {
    bytes.extend(id.as_uuid().as_bytes());
    text(bytes, email);
    bytes.extend(at.unix_timestamp_nanos().to_be_bytes());
    bytes.extend(at.offset().whole_seconds().to_be_bytes());
}
fn optional_text(bytes: &mut Vec<u8>, value: Option<&str>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        text(bytes, value);
    }
}
pub fn digest(family: u8, body: &[u8]) -> Sha256Digest {
    let mut bytes = b"DLOE1".to_vec();
    bytes.push(family);
    bytes.extend(body);
    inputs::hasher().hash_bytes(&bytes)
}
pub fn profile(value: &DeadlineProfileDetail) -> Vec<u8> {
    let mut bytes = vec![1];
    bytes.extend(value.id.as_uuid().as_bytes());
    bytes.extend(value.revision.get().to_be_bytes());
    bytes.extend(value.definition_digest.as_bytes());
    text(&mut bytes, value.status.as_str());
    optional_text(
        &mut bytes,
        value.reason.as_ref().map(|value| value.as_str()),
    );
    bytes.extend(value.receipt.operation_id.as_uuid().as_bytes());
    bytes.push(match value.receipt.action {
        DeadlineProfileAction::Publish => 0,
        DeadlineProfileAction::Replace => 1,
        DeadlineProfileAction::Retire => 2,
    });
    bytes.extend(value.receipt.expected_revision.to_be_bytes());
    bytes.extend(value.receipt.submission_digest.as_bytes());
    actor(
        &mut bytes,
        value.recorded_by.id,
        &value.recorded_by.email,
        value.recorded_at,
    );
    bytes
}
pub fn calendar(value: &JudicialCalendarDetail) -> Vec<u8> {
    let mut bytes = value.id.as_uuid().as_bytes().to_vec();
    bytes.extend(value.revision.get().to_be_bytes());
    bytes.extend(value.values_digest.as_bytes());
    text(&mut bytes, value.status.as_str());
    optional_text(
        &mut bytes,
        value.reason.as_ref().map(|value| value.as_str()),
    );
    bytes.extend(value.receipt.operation_id.as_uuid().as_bytes());
    bytes.push(match value.receipt.action {
        application::judicial_calendars::JudicialCalendarAction::Publish => 0,
        application::judicial_calendars::JudicialCalendarAction::Replace => 1,
        application::judicial_calendars::JudicialCalendarAction::Retire => 2,
    });
    bytes.extend(value.receipt.expected_revision.to_be_bytes());
    bytes.extend(value.receipt.submission_digest.as_bytes());
    actor(
        &mut bytes,
        value.recorded_by.id,
        &value.recorded_by.email,
        value.recorded_at,
    );
    bytes
}
pub fn fact(value: &FactDetail) -> Vec<u8> {
    // DLOE1 retains the existing inner source tag: resolution=1, notification=2.
    let mut bytes = Vec::new();
    match &value.snapshot {
        ProceduralFactSnapshot::Resolution(v) => {
            bytes.push(1);
            bytes.extend(v.root.id().as_uuid().as_bytes());
        }
        ProceduralFactSnapshot::Notification(v) => {
            bytes.push(2);
            bytes.extend(v.root.id().as_uuid().as_bytes());
            bytes.extend(v.root.resolution_id().as_uuid().as_bytes());
        }
    }
    bytes.extend(value.snapshot.case_id().as_uuid().as_bytes());
    let metadata = value.snapshot.metadata();
    bytes.extend(metadata.revision.get().to_be_bytes());
    bytes.extend(metadata.values_digest.as_bytes());
    bytes.push(match metadata.status {
        FactStatus::Recorded => 0,
        FactStatus::Withdrawn => 1,
    });
    optional_text(
        &mut bytes,
        metadata.reason.as_ref().map(|value| value.as_str()),
    );
    let receipt = &metadata.receipt;
    bytes.extend(receipt.operation_id.as_uuid().as_bytes());
    bytes.push(match receipt.action {
        FactAction::Record => 0,
        FactAction::Correct => 1,
        FactAction::Withdraw => 2,
    });
    bytes.extend(receipt.expected_revision.to_be_bytes());
    bytes.extend(receipt.sources_digest.as_bytes());
    bytes.extend(receipt.submission_digest.as_bytes());
    let admin = &metadata.recorded_administration;
    bytes.extend(case_administration_digest(inputs::hasher().as_ref(), &admin.values()).as_bytes());
    match admin {
        CurrentCaseAdministration::Unrevised(_) => bytes.push(0),
        CurrentCaseAdministration::Recorded(snapshot) => {
            bytes.push(1);
            bytes.extend(snapshot.case_id.as_uuid().as_bytes());
            bytes.extend(snapshot.revision.get().to_be_bytes());
            bytes.extend(snapshot.values_digest.as_bytes());
            actor(
                &mut bytes,
                snapshot.changed_by.id,
                &snapshot.changed_by.email,
                snapshot.changed_at,
            );
        }
    }
    actor(
        &mut bytes,
        metadata.recorded_by.id,
        &metadata.recorded_by.email,
        metadata.recorded_at,
    );
    bytes
}
