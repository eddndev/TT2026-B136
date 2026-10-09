use super::*;
use domain::{cases::CaseId, identity::UserId};

pub(super) fn capture(
    tx: &mut Transaction<'_>,
    report: &CaseReportDetail,
    cases: &[Uuid],
    now: OffsetDateTime,
) -> Result<CaseReportActivitySnapshot, ApplicationError> {
    let f = &report.command.filters;
    let visible = access::litigator_visibility(CaseReportKind::LitigatorActivity);
    let actors = tx
        .query(
            &format!(
                "SELECT u.id,u.email FROM users u WHERE {visible}
         AND ($3::uuid IS NULL OR u.id=$3) ORDER BY u.id LIMIT $4 FOR SHARE OF u"
            ),
            &[
                &(report.scope == CaseReportScope::Office),
                &report.requester.principal.id.as_uuid(),
                &f.litigator.map(UserId::as_uuid),
                &((MAX_REPORT_WORKLOAD + 1) as i64),
            ],
        )
        .map_err(port)?;
    if actors.len() > MAX_REPORT_WORKLOAD {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    let actors: Vec<_> = actors
        .into_iter()
        .map(|row| CaseReportLitigator {
            user_id: UserId::from_uuid(row.get("id")),
            email: row.get("email"),
        })
        .collect();
    let actor_ids: Vec<_> = actors.iter().map(|who| who.user_id.as_uuid()).collect();
    let sql = format!(
        "SELECT case_id,actor_id,
        count(*) FILTER(WHERE kind=0) AS documents,
        count(*) FILTER(WHERE kind=1) AS activities,
        count(*) FILTER(WHERE kind=2) AS deadlines
        FROM (SELECT DISTINCT case_id,actor_id,kind,identity FROM ({}) events
            WHERE case_id=ANY($1::uuid[]) AND actor_id=ANY($2::uuid[])
            AND (seconds,nanos)>=($3,$4) AND (seconds,nanos)<($5,$6)
            AND (seconds,nanos)<=($7,$8)) counted
        GROUP BY case_id,actor_id ORDER BY case_id,actor_id LIMIT $9",
        activity_sources::EVENTS
    );
    let rows = tx
        .query(
            &sql,
            &[
                &cases,
                &actor_ids,
                &f.period_from.unix_timestamp(),
                &(f.period_from.nanosecond() as i32),
                &f.period_before.unix_timestamp(),
                &(f.period_before.nanosecond() as i32),
                &now.unix_timestamp(),
                &(now.nanosecond() as i32),
                &((MAX_REPORT_ASSIGNMENTS + 1) as i64),
            ],
        )
        .map_err(port)?;
    if rows.len() > MAX_REPORT_ASSIGNMENTS {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    let rows = rows
        .into_iter()
        .map(|row| {
            Ok(CaseReportActivityRow {
                case_id: CaseId::from_uuid(row.get("case_id")),
                litigator_id: UserId::from_uuid(row.get("actor_id")),
                documents_uploaded: u64::try_from(row.get::<_, i64>("documents"))
                    .map_err(inconsistent)?,
                procedural_activities: u64::try_from(row.get::<_, i64>("activities"))
                    .map_err(inconsistent)?,
                deadlines_attended: u64::try_from(row.get::<_, i64>("deadlines"))
                    .map_err(inconsistent)?,
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    // A legacy upload lacks stable attribution even when its audit email still exists.
    let documents_complete = !tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM document_series s
        WHERE s.case_id=ANY($1::uuid[]) AND NOT EXISTS(
            SELECT 1 FROM document_upload_origins o WHERE o.document_id=s.id))",
            &[&cases],
        )
        .map_err(port)?
        .get::<_, bool>(0);
    Ok(CaseReportActivitySnapshot {
        actors,
        rows,
        documents_complete,
    })
}
