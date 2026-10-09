use super::*;

pub(super) const MAX_FIELD_BYTES: usize = 16 * 1024;
pub(super) const MAX_PAGES: usize = 512;
pub(super) const MAX_GLYPHS: usize = 1_000_000;

pub(super) fn capture(snapshot: &CaseReportSnapshot) -> Result<(), ApplicationError> {
    if snapshot.cases.len() > MAX_REPORT_CASES || snapshot.workload.len() > MAX_REPORT_WORKLOAD {
        return Err(capacity());
    }
    let mut bytes = 512usize;
    let mut assignments = 0usize;
    let mut field = |value: &str| -> Result<(), ApplicationError> {
        bytes = bytes.checked_add(value.len()).ok_or_else(capacity)?;
        if bytes > MAX_REPORT_SNAPSHOT_BYTES {
            return Err(capacity());
        }
        Ok(())
    };
    field(&snapshot.requester.principal.email)?;
    for row in &snapshot.cases {
        assignments = assignments
            .checked_add(row.assigned_litigators.len())
            .ok_or_else(capacity)?;
        if assignments > MAX_REPORT_ASSIGNMENTS {
            return Err(capacity());
        }
        field(&row.title)?;
        field(&row.reference)?;
        for who in &row.assigned_litigators {
            field(&who.email)?;
        }
    }
    for row in &snapshot.workload {
        field(&row.litigator.email)?;
    }
    let activity_overhead = if let Some(activity) = &snapshot.activity {
        if activity.actors.len() > MAX_REPORT_WORKLOAD
            || activity.rows.len() > MAX_REPORT_ASSIGNMENTS
        {
            return Err(capacity());
        }
        for who in &activity.actors {
            field(&who.email)?;
        }
        activity.actors.len() * 64 + activity.rows.len() * 64
    } else {
        0
    };
    let overhead = snapshot.cases.len() * 256
        + assignments * 64
        + snapshot.workload.len() * 96
        + activity_overhead;
    if bytes.checked_add(overhead).ok_or_else(capacity)? > MAX_REPORT_SNAPSHOT_BYTES {
        return Err(capacity());
    }
    Ok(())
}

pub(super) fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ApplicationError> {
    if out.len().checked_add(bytes.len()).ok_or_else(capacity)? > MAX_REPORT_ARTIFACT_BYTES {
        return Err(capacity());
    }
    out.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn utc(value: domain::clock::OffsetDateTime) -> Result<String, ApplicationError> {
    value
        .to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| failed())
}

pub(super) fn scope(value: CaseReportScope) -> &'static str {
    match value {
        CaseReportScope::Office => "office",
        CaseReportScope::AssignedCases => "assigned_cases",
    }
}

pub(super) fn status(value: application::cases::CaseStatusFilter) -> &'static str {
    match value {
        application::cases::CaseStatusFilter::All => "all",
        application::cases::CaseStatusFilter::Active => "active",
        application::cases::CaseStatusFilter::Closed => "closed",
    }
}
