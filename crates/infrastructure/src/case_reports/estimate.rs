use super::*;
use time::Duration;

/// Metadata only: no snapshot decryption or cross-account report projection.
pub(super) fn initial(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    scope: CaseReportScope,
    command: &CaseReportCommand,
    id: CaseReportId,
    at: OffsetDateTime,
) -> Result<(Option<i64>, Option<i64>), ApplicationError> {
    let ids = match selection::case_ids(tx, actor, scope, &command.filters, at) {
        Ok(ids) => ids,
        Err(ApplicationError::CaseReport(CaseReportError::CapacityExceeded)) => {
            return Ok((None, None))
        }
        Err(error) => return Err(error),
    };
    let kind = match command.filters.kind {
        CaseReportKind::CaseState => "case_state",
        CaseReportKind::LitigatorActivity => "litigator_activity",
    };
    let rows = tx
        .query(
            "SELECT cardinality(j.case_ids), s.checked_at, n.created_at
        FROM case_report_jobs j JOIN case_report_snapshots s ON s.report_id=j.id
        JOIN case_report_notices n ON n.report_id=j.id
        WHERE j.state='ready' AND n.kind='ready' AND j.scope=$1 AND j.attempts=1
        AND coalesce(j.command->'filters'->>'kind','case_state')=$2
        AND n.created_at::timestamptz < $3::text::timestamptz
        ORDER BY n.created_at::timestamptz DESC, j.id LIMIT 64",
            &[&scope_name(scope), &kind, &timestamp(at)?],
        )
        .map_err(port)?;
    let samples = rows
        .iter()
        .map(|row| {
            let start = parse_time(row.get(1))?;
            let end = parse_time(row.get(2))?;
            Ok(CaseReportTimingSample {
                cases: u32::try_from(row.get::<_, i32>(0)).map_err(inconsistent)?,
                service_ms: u64::try_from((end - start).whole_milliseconds().max(0))
                    .map_err(inconsistent)?,
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    let Some(service) = estimate_report_service_ms(ids.len() as u32, &samples)
        .and_then(|value| i64::try_from(value).ok())
    else {
        return Ok((None, None));
    };
    Ok((forecast(tx, id, service, at)?, Some(service)))
}

fn forecast(
    tx: &mut Transaction<'_>,
    id: CaseReportId,
    service: i64,
    at: OffsetDateTime,
) -> Result<Option<i64>, ApplicationError> {
    let rows = tx
        .query(
            "SELECT j.id,j.state,j.attempts,j.estimated_service_ms,j.retry_at,j.lease_expires_at,
        coalesce(s.checked_at,j.updated_at) AS started_at FROM case_report_jobs j
        LEFT JOIN case_report_snapshots s ON s.report_id=j.id
        WHERE j.state IN ('queued','capturing','rendering','retry_capturing','retry_rendering')
        AND (j.id < $1 OR j.state IN ('capturing','rendering')) ORDER BY j.id LIMIT 101",
            &[&id.as_uuid()],
        )
        .map_err(port)?;
    if rows.len() > 100 {
        return Ok(None);
    }
    // The serial supervisor waits one second before its next claim. Reserve the
    // whole interval rather than presenting an unobserved fractional poll delay.
    let mut wait = 1_000i64;
    for row in &rows {
        let lease = row
            .get::<_, Option<&str>>("lease_expires_at")
            .map(parse_time)
            .transpose()?;
        if lease.is_some_and(|value| value > at) {
            if row.get::<_, i32>("attempts") > 1 {
                return Ok(None);
            }
            let Some(cost) = row.get::<_, Option<i64>>("estimated_service_ms") else {
                return Ok(None);
            };
            let started = parse_time(row.get("started_at"))?;
            let elapsed =
                i64::try_from((at - started).whole_milliseconds().max(0)).map_err(inconsistent)?;
            wait = wait
                .checked_add(cost.saturating_sub(elapsed).max(0))
                .ok_or_else(|| inconsistent("report queue duration overflow"))?;
        }
    }
    let mut pending = rows
        .iter()
        .filter(|row| row.get::<_, Uuid>("id") < id.as_uuid())
        .collect::<Vec<_>>();
    loop {
        let mut next = None;
        for (index, row) in pending.iter().enumerate() {
            let lease = row
                .get::<_, Option<&str>>("lease_expires_at")
                .map(parse_time)
                .transpose()?;
            if lease.is_some_and(|value| value > at) {
                continue;
            }
            let available = row
                .get::<_, Option<&str>>("retry_at")
                .map(parse_time)
                .transpose()?;
            let Some(claim_at) = at.checked_add(Duration::milliseconds(wait)) else {
                return Ok(None);
            };
            if available.is_none_or(|value| value <= claim_at) {
                next = Some(index);
                break;
            }
        }
        let Some(index) = next else {
            break;
        };
        let row = pending.remove(index);
        let Some(cost) = row.get::<_, Option<i64>>("estimated_service_ms") else {
            return Ok(None);
        };
        wait = wait
            .checked_add(cost)
            .and_then(|value| value.checked_add(1_000))
            .ok_or_else(|| inconsistent("report queue duration overflow"))?;
    }
    Ok(wait.checked_add(service))
}
