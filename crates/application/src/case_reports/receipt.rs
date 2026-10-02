use super::{checks::*, *};
use crate::{identity::Principal, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};

pub(super) fn detail(
    hasher: &dyn DocumentHasher,
    value: &CaseReportDetail,
    actor: &Principal,
    id: CaseReportId,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    requester(&value.requester, value.scope)?;
    time(value.requested_at)?;
    time(value.updated_at)?;
    if value.id != id
        || id.as_uuid().is_nil()
        || value.requester.principal != *actor
        || value.requested_at > value.updated_at
        || value.updated_at > at
    {
        return Err(inconsistent(
            "report receipt identity, requester or clock differs",
        ));
    }
    let digest = case_report_request_digest(hasher, actor, value.scope, &value.command)
        .map_err(|_| inconsistent("report request capture is invalid"))?;
    if digest != value.request_digest {
        return Err(inconsistent("report request digest differs"));
    }
    let expected_notice = match &value.state {
        CaseReportState::Ready {
            checked_at,
            artifacts,
            ..
        } => {
            time(*checked_at)?;
            if *checked_at < value.requested_at || *checked_at > value.updated_at {
                return Err(inconsistent("report capture time is outside its lifetime"));
            }
            metadata(artifacts)?;
            Some(CaseReportNoticeKind::Ready)
        }
        CaseReportState::Failed(_) => Some(CaseReportNoticeKind::Failed),
        CaseReportState::RetryWaiting { retry_at, .. } => {
            time(*retry_at)?;
            if *retry_at < value.updated_at {
                return Err(inconsistent("report retry predates its transition"));
            }
            None
        }
        CaseReportState::AccessRevoked => return Err(CaseReportError::AccessRevoked.into()),
        _ => None,
    };
    match (&value.notice, expected_notice) {
        (None, None) => {}
        (Some(notice), Some(kind)) if notice.kind == kind => {
            time(notice.created_at)?;
            let earliest = match value.state {
                CaseReportState::Ready { checked_at, .. } => checked_at,
                _ => value.requested_at,
            };
            if notice.created_at < earliest || notice.created_at > value.updated_at {
                return Err(inconsistent(
                    "report notice time differs from terminal state",
                ));
            }
            if let Some(read_at) = notice.read_at {
                time(read_at)?;
                if read_at < notice.created_at || read_at > value.updated_at {
                    return Err(inconsistent("report notice acknowledgement time differs"));
                }
            }
        }
        _ => return Err(inconsistent("report notice differs from terminal state")),
    }
    Ok(())
}
pub(super) fn metadata(values: &[CaseReportArtifactMetadata]) -> Result<(), ApplicationError> {
    if values.len() != 2
        || values[0].format == values[1].format
        || values
            .iter()
            .any(|v| v.bytes == 0 || v.bytes > MAX_REPORT_ARTIFACT_BYTES as u64)
    {
        return Err(inconsistent(
            "report requires exactly two bounded format artifacts",
        ));
    }
    Ok(())
}
pub(super) fn download(
    hasher: &dyn DocumentHasher,
    value: &CaseReportDownload,
    actor: &Principal,
    id: CaseReportId,
    format: CaseReportFormat,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    detail(hasher, &value.report, actor, id, at)?;
    let CaseReportState::Ready {
        snapshot_digest,
        artifacts,
        ..
    } = &value.report.state
    else {
        return Err(inconsistent(
            "download returned bytes before report completion",
        ));
    };
    let artifact = &value.artifact;
    if artifact.report_id != id
        || artifact.format != format
        || artifact.requester != value.report.requester
        || artifact.scope != value.report.scope
        || artifact.snapshot_digest != *snapshot_digest
        || artifact.content.is_empty()
        || artifact.content.len() > MAX_REPORT_ARTIFACT_BYTES
    {
        return Err(inconsistent(
            "download artifact identity, capture or bounds differ",
        ));
    }
    let metadata = artifacts
        .iter()
        .find(|v| v.format == format)
        .ok_or_else(|| inconsistent("download format has no ready metadata"))?;
    if metadata.bytes != artifact.content.len() as u64
        || metadata.digest != artifact.digest
        || hasher.hash_bytes(&artifact.content) != artifact.digest
    {
        return Err(inconsistent(
            "download bytes differ from their ready digest",
        ));
    }
    Ok(())
}
pub(super) fn capture(
    hasher: &dyn DocumentHasher,
    snapshot: &CaseReportSnapshot,
    report: &CaseReportDetail,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    validate_case_report_snapshot(hasher, snapshot)?;
    if snapshot.report_id != report.id
        || snapshot.requester != report.requester
        || snapshot.scope != report.scope
        || snapshot.filters != report.command.filters
        || snapshot.checked_at < report.requested_at
        || snapshot.checked_at > at
    {
        return Err(inconsistent(
            "report capture belongs to another request or clock",
        ));
    }
    Ok(())
}
pub(super) fn completion(
    hasher: &dyn DocumentHasher,
    result: &CaseReportDetail,
    original: &CaseReportDetail,
    capture: &CaseReportSnapshot,
    artifacts: &[CaseReportArtifactMetadata],
    lease: &CaseReportLease,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    detail(
        hasher,
        result,
        &original.requester.principal,
        original.id,
        at,
    )?;
    if result.requester != original.requester
        || result.command != original.command
        || result.request_digest != original.request_digest
        || result.scope != original.scope
        || result.requested_at != original.requested_at
        || result.updated_at >= lease.expires_at
    {
        return Err(inconsistent(
            "completion differs from its claimed request or valid lease",
        ));
    }
    let CaseReportState::Ready {
        snapshot_digest,
        checked_at,
        artifacts: saved,
    } = &result.state
    else {
        return Err(inconsistent("completion did not return Ready"));
    };
    if *snapshot_digest != capture.digest
        || *checked_at != capture.checked_at
        || saved.len() != artifacts.len()
        || artifacts.iter().any(|expected| !saved.contains(expected))
        || result
            .notice
            .as_ref()
            .is_none_or(|notice| notice.read_at.is_some())
    {
        return Err(inconsistent(
            "completion differs from the rendered capture or artifacts",
        ));
    }
    Ok(())
}
