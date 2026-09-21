use super::*;
use crate::{cases::case_administration_digest, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};

/// Receipt framing, independent of storage JSON and current target projections.
pub fn resource_activity_submission_bytes(
    hasher: &dyn DocumentHasher,
    draft: &ResourceActivityDraft,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"RATX1".to_vec();
    bytes.extend_from_slice(draft.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(draft.resource_id.as_uuid().as_bytes());
    bytes.extend_from_slice(draft.command.association_id.as_uuid().as_bytes());
    bytes.extend_from_slice(draft.command.operation_id.as_uuid().as_bytes());
    bytes.push(draft.command.action().tag());
    bytes.extend_from_slice(&draft.command.expected_revision().to_be_bytes());
    bytes.extend_from_slice(&draft.command.expected_resource_revision.get().to_be_bytes());
    bytes.extend_from_slice(&draft.result_revision.get().to_be_bytes());
    bytes.push(draft.status.tag());
    bytes.extend_from_slice(&draft.selection.canonical_bytes());
    bytes.extend_from_slice(draft.observed_resource_head.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&draft.observed_resource_head.revision.get().to_be_bytes());
    bytes.extend_from_slice(draft.observed_resource_head.capture_digest.as_bytes());
    bytes.push(u8::from(draft.previous.is_some()));
    if let Some(previous) = draft.previous {
        bytes.extend_from_slice(&previous.revision.get().to_be_bytes());
        bytes.extend_from_slice(previous.capture_digest.as_bytes());
    }
    bytes.push(u8::from(draft.command.reason().is_some()));
    if let Some(reason) = draft.command.reason() {
        blob(&mut bytes, reason.as_str().as_bytes());
    }
    bytes.extend_from_slice(draft.recorded_by.id.as_uuid().as_bytes());
    blob(&mut bytes, draft.recorded_by.email.as_bytes());
    let admin = &draft.observed_administration;
    bytes.extend_from_slice(case_administration_digest(hasher, &admin.values()).as_bytes());
    bytes.push(u8::from(admin.snapshot().is_some()));
    if let Some(snapshot) = admin.snapshot() {
        bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
        bytes.extend_from_slice(snapshot.values_digest.as_bytes());
        bytes.extend_from_slice(snapshot.changed_by.id.as_uuid().as_bytes());
        blob(&mut bytes, snapshot.changed_by.email.as_bytes());
        instant(&mut bytes, snapshot.changed_at);
    }
    Ok(bytes)
}
pub fn resource_activity_capture_bytes(detail: &ResourceActivityDetail) -> Vec<u8> {
    let mut bytes = b"RACP1".to_vec();
    bytes.extend_from_slice(detail.receipt.submission_digest.as_bytes());
    bytes.extend_from_slice(&detail.recorded_at.unix_timestamp().to_be_bytes());
    bytes.extend_from_slice(&detail.recorded_at.nanosecond().to_be_bytes());
    bytes
}
pub(super) fn draft_from_detail(
    detail: &ResourceActivityDetail,
) -> Result<ResourceActivityDraft, ApplicationError> {
    Ok(ResourceActivityDraft {
        case_id: detail.case_id,
        resource_id: detail.resource_id,
        command: resource_activity_command_from_detail(detail)?,
        result_revision: detail.revision,
        selection: detail.selection,
        status: detail.status,
        sources: detail.sources.clone(),
        previous: detail.receipt.previous,
        recorded_by: detail.recorded_by.clone(),
        observed_administration: detail.recorded_administration.clone(),
        observed_resource_head: detail.recorded_resource_head,
        submission_digest: detail.receipt.submission_digest,
    })
}
fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}
fn instant(bytes: &mut Vec<u8>, at: OffsetDateTime) {
    bytes.extend_from_slice(&at.unix_timestamp().to_be_bytes());
    bytes.extend_from_slice(&at.nanosecond().to_be_bytes());
    bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
}
