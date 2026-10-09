use super::*;

const HEADER: &str = "row_type,report_id,snapshot_digest,checked_at,requester_id,scope,report_kind,period_from,period_before,status_filter,litigator_filter,documents_complete,case_id,title,reference,created_at,status,litigator_id,litigator_email,documents_uploaded,procedural_activities,deadlines_attended\r\n";

fn counts(row: &mut [String], values: [u64; 3]) {
    for (index, value) in values.into_iter().enumerate() {
        row[19 + index] = value.to_string();
    }
}

pub(super) fn render(snapshot: &CaseReportSnapshot) -> Result<Vec<u8>, ApplicationError> {
    let value = activity::payload(snapshot)?;
    let totals = activity::totals(value)?;
    let mut base = vec![String::new(); 22];
    base[1] = snapshot.report_id.to_string();
    base[2] = snapshot.digest.to_string();
    base[3] = bounds::utc(snapshot.checked_at)?;
    base[4] = snapshot.requester.principal.id.to_string();
    base[5] = bounds::scope(snapshot.scope).into();
    base[6] = "litigator_activity".into();
    base[7] = bounds::utc(snapshot.filters.period_from)?;
    base[8] = bounds::utc(snapshot.filters.period_before)?;
    base[9] = bounds::status(snapshot.filters.status).into();
    base[10] = snapshot
        .filters
        .litigator
        .map(|id| id.to_string())
        .unwrap_or_default();
    base[11] = value.documents_complete.to_string();
    let mut out = Vec::new();
    bounds::append(&mut out, HEADER.as_bytes())?;
    let mut row = base.clone();
    row[0] = "capture".into();
    counts(&mut row, totals.capture);
    csv::record(&mut out, &row)?;
    for (who, total) in value.actors.iter().zip(totals.actors) {
        let mut row = base.clone();
        row[0] = "litigator".into();
        row[17] = who.user_id.to_string();
        row[18] = format!("'{}", who.email);
        counts(&mut row, total);
        csv::record(&mut out, &row)?;
    }
    for entry in &value.rows {
        let case = activity::case(snapshot, entry.case_id)?;
        let who = activity::actor(value, entry.litigator_id)?;
        let mut row = base.clone();
        row[0] = "case_activity".into();
        row[12] = case.case_id.to_string();
        row[13] = format!("'{}", case.title);
        row[14] = format!("'{}", case.reference);
        row[15] = bounds::utc(case.created_at)?;
        row[16] = case.status.as_str().into();
        row[17] = who.user_id.to_string();
        row[18] = format!("'{}", who.email);
        counts(&mut row, activity::counts(entry));
        csv::record(&mut out, &row)?;
    }
    Ok(out)
}
