use super::*;
use domain::case_administration::CaseAdministrativeStatus;

const HEADER: &str = "row_type,report_id,snapshot_digest,checked_at,requester_id,scope,created_from,created_before,status_filter,assigned_litigator_filter,case_id,title,reference,created_at,status,administration_revision,administration_digest,assigned_litigators,litigator_id,litigator_email,active_cases,closed_cases,total_cases\r\n";

pub(super) fn record(out: &mut Vec<u8>, cells: &[String]) -> Result<(), ApplicationError> {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            bounds::append(out, b",")?;
        }
        let quote = cell
            .bytes()
            .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'));
        if quote {
            bounds::append(out, b"\"")?;
        }
        let mut remainder = cell.as_str();
        while let Some(at) = remainder.find('"') {
            bounds::append(out, &remainder.as_bytes()[..at])?;
            bounds::append(out, b"\"\"")?;
            remainder = &remainder[at + 1..];
        }
        bounds::append(out, remainder.as_bytes())?;
        if quote {
            bounds::append(out, b"\"")?;
        }
    }
    bounds::append(out, b"\r\n")
}

pub(super) fn render(snapshot: &CaseReportSnapshot) -> Result<Vec<u8>, ApplicationError> {
    let mut base = vec![String::new(); 23];
    base[1] = snapshot.report_id.to_string();
    base[2] = snapshot.digest.to_string();
    base[3] = bounds::utc(snapshot.checked_at)?;
    base[4] = snapshot.requester.principal.id.to_string();
    base[5] = bounds::scope(snapshot.scope).into();
    base[6] = bounds::utc(snapshot.filters.period_from)?;
    base[7] = bounds::utc(snapshot.filters.period_before)?;
    base[8] = bounds::status(snapshot.filters.status).into();
    base[9] = snapshot
        .filters
        .litigator
        .map(|id| id.to_string())
        .unwrap_or_default();
    let mut out = Vec::new();
    bounds::append(&mut out, HEADER.as_bytes())?;
    let mut row = base.clone();
    row[0] = "capture".into();
    let active = snapshot
        .cases
        .iter()
        .filter(|case| case.status == CaseAdministrativeStatus::Active)
        .count();
    row[20] = active.to_string();
    row[21] = (snapshot.cases.len() - active).to_string();
    row[22] = snapshot.cases.len().to_string();
    record(&mut out, &row)?;
    for case in &snapshot.cases {
        let mut row = base.clone();
        row[0] = "case".into();
        row[10] = case.case_id.to_string();
        row[11] = format!("'{}", case.title);
        row[12] = format!("'{}", case.reference);
        row[13] = bounds::utc(case.created_at)?;
        row[14] = case.status.as_str().into();
        row[15] = case
            .administration_revision
            .map(|revision| revision.get().to_string())
            .unwrap_or_default();
        row[16] = case
            .administration_digest
            .map(|digest| digest.to_string())
            .unwrap_or_default();
        let assignments: Vec<_> = case
            .assigned_litigators
            .iter()
            .map(|who| serde_json::json!({"user_id": who.user_id.to_string(), "email": who.email}))
            .collect();
        row[17] = format!(
            "'{}",
            serde_json::to_string(&assignments).map_err(|_| failed())?
        );
        record(&mut out, &row)?;
    }
    for workload in &snapshot.workload {
        let mut row = base.clone();
        row[0] = "workload".into();
        row[18] = workload.litigator.user_id.to_string();
        row[19] = format!("'{}", workload.litigator.email);
        row[20] = workload.active_cases.to_string();
        row[21] = workload.closed_cases.to_string();
        row[22] = workload
            .active_cases
            .checked_add(workload.closed_cases)
            .ok_or_else(capacity)?
            .to_string();
        record(&mut out, &row)?;
    }
    Ok(out)
}
