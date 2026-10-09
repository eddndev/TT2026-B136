use crate::error::ApiError;
use application::{case_reports::*, cases::CaseStatusFilter};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub(super) fn at(value: OffsetDateTime) -> Result<String, ApiError> {
    value.format(&Rfc3339).map_err(|_| ApiError::internal())
}
pub(super) fn phase(value: CaseReportPhase) -> &'static str {
    match value {
        CaseReportPhase::Capturing => "capturing",
        CaseReportPhase::Rendering => "rendering",
    }
}
pub(super) fn format(value: CaseReportFormat) -> &'static str {
    match value {
        CaseReportFormat::Pdf => "pdf",
        CaseReportFormat::Csv => "csv",
    }
}
pub(super) fn failure(value: CaseReportFailure) -> &'static str {
    match value {
        CaseReportFailure::TemporaryUnavailable => "temporarily_unavailable",
        CaseReportFailure::RenderUnavailable => "render_unavailable",
        CaseReportFailure::RenderFailed => "render_failed",
        CaseReportFailure::CapacityExceeded => "capacity_exceeded",
        CaseReportFailure::InvalidStoredCapture => "invalid_capture",
        CaseReportFailure::AccessRevoked => "access_revoked",
    }
}
pub(super) fn detail(value: CaseReportDetail) -> Result<Value, ApiError> {
    let (state, phase, retry, failure, ready) = match value.state {
        CaseReportState::Queued => ("queued", None, None, None, None),
        CaseReportState::Processing(p) => ("processing", Some(phase(p)), None, None, None),
        CaseReportState::RetryWaiting { phase: p, retry_at } => (
            "retry_waiting",
            Some(phase(p)),
            Some(at(retry_at)?),
            None,
            None,
        ),
        CaseReportState::Ready {
            snapshot_digest,
            checked_at,
            artifacts,
        } => (
            "ready",
            None,
            None,
            None,
            Some(json!({
            "snapshot_digest":snapshot_digest.to_hex(),"checked_at":at(checked_at)?,
            "artifacts":artifacts.into_iter().map(|v|json!({"format":format(v.format),"bytes":v.bytes,"digest":v.digest.to_hex()})).collect::<Vec<_>>() })),
        ),
        CaseReportState::Failed(reason) => ("failed", None, None, Some(failure(reason)), None),
        CaseReportState::AccessRevoked => {
            ("access_revoked", None, None, Some("access_revoked"), None)
        }
    };
    let notice=value.notice.map(|v|->Result<Value,ApiError>{Ok(json!({"kind":match v.kind{CaseReportNoticeKind::Ready=>"ready",CaseReportNoticeKind::Failed=>"failed"},
        "created_at":at(v.created_at)?,"read_at":v.read_at.map(at).transpose()?}))}).transpose()?;
    let filters = value.command.filters;
    let mut result = json!({"id":value.id.to_string(),"operation_id":value.command.operation_id.to_string(),
        "request_digest":value.request_digest.to_hex(),"scope":match value.scope{CaseReportScope::Office=>"office",CaseReportScope::AssignedCases=>"assigned_cases"},
        "requested_at":at(value.requested_at)?,"updated_at":at(value.updated_at)?,
        "state":state,"phase":phase,"retry_at":retry,"failure":failure,"ready":ready,"notice":notice});
    result["filters"] = filters_value(&filters)?;
    if filters.kind == CaseReportKind::LitigatorActivity {
        result["report_type"] = json!("litigator_activity");
    }
    Ok(result)
}

pub(super) fn litigators(value: CaseReportLitigatorPage) -> Result<Value, ApiError> {
    Ok(json!({
        "scope": match value.scope { CaseReportScope::Office => "office", CaseReportScope::AssignedCases => "assigned_cases" },
        "checked_at": at(value.checked_at)?,
        "litigators": value.litigators.into_iter().map(|member| json!({
            "user_id": member.user_id.to_string(), "email": member.email
        })).collect::<Vec<_>>(),
        "has_more": value.has_more,
        "next_after_id": value.next_after_id.map(|id| id.to_string())
    }))
}

fn filters_value(filters: &CaseReportFilters) -> Result<Value, ApiError> {
    let status = match filters.status {
        CaseStatusFilter::All => "all",
        CaseStatusFilter::Active => "active",
        CaseStatusFilter::Closed => "closed",
    };
    let from = at(filters.period_from)?;
    let before = at(filters.period_before)?;
    let who = filters.litigator.map(|id| id.to_string());
    Ok(match filters.kind {
        CaseReportKind::CaseState => json!({"created_from":from,"created_before":before,
            "status":status,"assigned_litigator":who}),
        CaseReportKind::LitigatorActivity => json!({"occurred_from":from,"occurred_before":before,
            "status":status,"author_litigator":who}),
    })
}
