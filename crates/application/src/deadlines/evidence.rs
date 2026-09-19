use super::*;
use crate::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::DeadlineProfileDetail,
    judicial_calendars::JudicialCalendarDetail,
    procedural_facts::{FactAction, FactStatus, ProceduralFactSnapshot},
};
use domain::{crypto::DocumentHasher, identity::UserId};
use time::OffsetDateTime;

/// Internal evidence strings are preserved literally, with UTF-8 byte framing.
pub(super) fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
pub(super) fn optional<T>(
    bytes: &mut Vec<u8>,
    value: Option<T>,
    encode: impl FnOnce(&mut Vec<u8>, T),
) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        encode(bytes, value);
    }
}
pub(super) fn instant(bytes: &mut Vec<u8>, at: OffsetDateTime) {
    bytes.extend_from_slice(&at.unix_timestamp_nanos().to_be_bytes());
    bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
}
pub(super) fn actor(bytes: &mut Vec<u8>, id: UserId, email: &str, at: OffsetDateTime) {
    bytes.extend_from_slice(id.as_uuid().as_bytes());
    text(bytes, email);
    instant(bytes, at);
}
pub(crate) fn administration(
    bytes: &mut Vec<u8>,
    hasher: &dyn DocumentHasher,
    admin: &CurrentCaseAdministration,
) {
    // CADM1 covers all values, including the exact original metadata of R0.
    bytes.extend_from_slice(case_administration_digest(hasher, &admin.values()).as_bytes());
    optional(bytes, admin.snapshot(), |bytes, snapshot| {
        bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
        bytes.extend_from_slice(snapshot.values_digest.as_bytes());
        actor(
            bytes,
            snapshot.changed_by.id,
            &snapshot.changed_by.email,
            snapshot.changed_at,
        );
    });
}
/// Callers first verify each canonical values/source digest against its full payload.
/// Those commitments cover DPRF1, JCAL1, PFRES1/PFNOT1, PFSRC1 and HRES1, while
/// every additional captured field is encoded explicitly. Old operation receipts
/// alone do not bind historical projections or administrative metadata.
pub(super) fn calculation(
    bytes: &mut Vec<u8>,
    hasher: &dyn DocumentHasher,
    value: &DeadlineCalculation,
    include_observed_administration: bool,
) {
    profile(bytes, &value.profile);
    let material = &value.material;
    bytes.extend_from_slice(material.case_id.as_uuid().as_bytes());
    // Observed administration may advance while the confirmed content stays equal.
    // Historical administration inside a source remains fixed in both digests.
    if include_observed_administration {
        administration(bytes, hasher, &material.administration);
    }
    source(bytes, hasher, material.source.as_ref());
    source(bytes, hasher, material.source_head.as_ref());
    for calendar in [&material.calendar, &material.calendar_head] {
        optional(bytes, calendar.as_ref(), self::calendar);
    }
}
/// The caller verifies the definition digest before committing captured metadata.
pub(crate) fn profile(bytes: &mut Vec<u8>, profile: &DeadlineProfileDetail) {
    bytes.push(profile.algorithm.tag());
    bytes.extend_from_slice(profile.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&profile.revision.get().to_be_bytes());
    bytes.extend_from_slice(profile.definition_digest.as_bytes());
    text(bytes, profile.status.as_str());
    optional(bytes, profile.reason.as_ref(), |bytes, reason| {
        text(bytes, reason.as_str())
    });
    bytes.extend_from_slice(profile.receipt.operation_id.as_uuid().as_bytes());
    bytes.push(profile.receipt.action.tag());
    bytes.extend_from_slice(&profile.receipt.expected_revision.to_be_bytes());
    bytes.extend_from_slice(profile.receipt.submission_digest.as_bytes());
    actor(
        bytes,
        profile.recorded_by.id,
        &profile.recorded_by.email,
        profile.recorded_at,
    );
}
/// The caller verifies the calendar values digest before committing captured metadata.
pub(crate) fn calendar(bytes: &mut Vec<u8>, calendar: &JudicialCalendarDetail) {
    bytes.extend_from_slice(calendar.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&calendar.revision.get().to_be_bytes());
    bytes.extend_from_slice(calendar.values_digest.as_bytes());
    text(bytes, calendar.status.as_str());
    optional(bytes, calendar.reason.as_ref(), |bytes, reason| {
        text(bytes, reason.as_str())
    });
    bytes.extend_from_slice(calendar.receipt.operation_id.as_uuid().as_bytes());
    bytes.push(calendar.receipt.action.tag());
    bytes.extend_from_slice(&calendar.receipt.expected_revision.to_be_bytes());
    bytes.extend_from_slice(calendar.receipt.submission_digest.as_bytes());
    actor(
        bytes,
        calendar.recorded_by.id,
        &calendar.recorded_by.email,
        calendar.recorded_at,
    );
}

pub(crate) fn source(
    bytes: &mut Vec<u8>,
    hasher: &dyn DocumentHasher,
    value: Option<&DeadlineSourceDetail>,
) {
    let Some(value) = value else {
        bytes.push(0);
        return;
    };
    match value {
        DeadlineSourceDetail::Fact(detail) => {
            match &detail.snapshot {
                ProceduralFactSnapshot::Resolution(value) => {
                    bytes.push(1);
                    bytes.extend_from_slice(value.root.id().as_uuid().as_bytes());
                }
                ProceduralFactSnapshot::Notification(value) => {
                    bytes.push(2);
                    bytes.extend_from_slice(value.root.id().as_uuid().as_bytes());
                    bytes.extend_from_slice(value.root.resolution_id().as_uuid().as_bytes());
                }
            }
            let metadata = detail.snapshot.metadata();
            bytes.extend_from_slice(detail.snapshot.case_id().as_uuid().as_bytes());
            bytes.extend_from_slice(&metadata.revision.get().to_be_bytes());
            bytes.extend_from_slice(metadata.values_digest.as_bytes());
            bytes.push(match metadata.status {
                FactStatus::Recorded => 0,
                FactStatus::Withdrawn => 1,
            });
            optional(bytes, metadata.reason.as_ref(), |bytes, reason| {
                text(bytes, reason.as_str())
            });
            bytes.extend_from_slice(metadata.receipt.operation_id.as_uuid().as_bytes());
            bytes.push(match metadata.receipt.action {
                FactAction::Record => 0,
                FactAction::Correct => 1,
                FactAction::Withdraw => 2,
            });
            bytes.extend_from_slice(&metadata.receipt.expected_revision.to_be_bytes());
            bytes.extend_from_slice(metadata.receipt.sources_digest.as_bytes());
            bytes.extend_from_slice(metadata.receipt.submission_digest.as_bytes());
            administration(bytes, hasher, &metadata.recorded_administration);
            actor(
                bytes,
                metadata.recorded_by.id,
                &metadata.recorded_by.email,
                metadata.recorded_at,
            );
        }
        DeadlineSourceDetail::HearingResult(detail) => {
            bytes.push(3);
            super::evidence_hearing::hearing(bytes, detail);
        }
    }
}
