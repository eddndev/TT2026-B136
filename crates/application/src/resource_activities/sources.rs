use super::*;
use crate::{
    deadlines::deadline_receipt_matches,
    hearings::hearing_receipt_matches,
    procedural_resources::{resource_receipt_matches, ResourceDetail},
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};

pub(super) fn resource_ref(row: &ResourceDetail) -> ResourceCaptureRef {
    ResourceCaptureRef {
        id: row.id,
        revision: row.revision,
        capture_digest: row.receipt.capture_digest,
    }
}
pub(super) fn verify(
    hasher: &dyn DocumentHasher,
    case: CaseId,
    resource: ResourceId,
    selection: ResourceActivitySelection,
    sources: &ResourceActivitySources,
) -> Result<(), ApplicationError> {
    resource_receipt_matches(hasher, &sources.resource)?;
    if sources.resource.case_id != case
        || sources.resource.id != resource
        || selection.resource != resource_ref(&sources.resource)
    {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    match (selection.act, &sources.act) {
        (None, None) => {}
        (Some(reference), Some(row)) => {
            resource_receipt_matches(hasher, row)?;
            let act = row
                .act
                .as_ref()
                .ok_or_else(|| inconsistent("selected revision contains no act"))?;
            if row.case_id != case
                || row.id != resource
                || row.revision != reference.resource_revision
                || row.receipt.capture_digest != reference.capture_digest
                || act.id != reference.id
                || act.revision != reference.revision
            {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
        }
        _ => return Err(ResourceActivityError::SourceMismatch.into()),
    }
    match (selection.target, &sources.target) {
        (
            ResourceActivityTarget::Hearing {
                id,
                revision,
                submission_digest,
            },
            ResourceActivityTargetDetail::Hearing(row),
        ) => {
            hearing_receipt_matches(hasher, row)?;
            if row.snapshot.case_id != case
                || row.snapshot.id != id
                || row.snapshot.revision != revision
                || row.snapshot.receipt.submission_digest != submission_digest
            {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
        }
        (
            ResourceActivityTarget::Deadline {
                id,
                revision,
                capture_digest,
            },
            ResourceActivityTargetDetail::Deadline(row),
        ) => {
            deadline_receipt_matches(hasher, row)?;
            if row.case_id != case
                || row.id != id
                || row.revision != revision
                || row.receipt.capture_digest != capture_digest
            {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
        }
        _ => return Err(ResourceActivityError::SourceMismatch.into()),
    }
    Ok(())
}
pub(super) fn latest_time(sources: &ResourceActivitySources) -> OffsetDateTime {
    let target = match &sources.target {
        ResourceActivityTargetDetail::Hearing(value) => value.snapshot.recorded_at,
        ResourceActivityTargetDetail::Deadline(value) => value.recorded_at,
    };
    let act = sources
        .act
        .as_ref()
        .map(|value| value.recorded_at)
        .unwrap_or(sources.resource.recorded_at);
    target.max(act).max(sources.resource.recorded_at)
}
