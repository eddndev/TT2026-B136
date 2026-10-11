use super::*;
use application::cases::CaseStatusFilter;
use domain::identity::UserId;

pub(super) fn case_ids(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    scope: CaseReportScope,
    f: &CaseReportFilters,
    now: OffsetDateTime,
) -> Result<Vec<Uuid>, ApplicationError> {
    let status = match f.status {
        CaseStatusFilter::All => "all",
        CaseStatusFilter::Active => "active",
        CaseStatusFilter::Closed => "closed",
    };
    let rows = tx.query("SELECT c.id FROM cases c LEFT JOIN LATERAL
            (SELECT administrative_status FROM case_administration_revisions r WHERE r.case_id=c.id ORDER BY revision DESC LIMIT 1) h ON true
            WHERE ($9::boolean OR (c.created_at >= $1::text::timestamptz AND c.created_at < $2::text::timestamptz))
            AND c.created_at <= $3::text::timestamptz
            AND ($4::boolean OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=c.id AND m.user_id=$5))
            AND ($6='all' OR coalesce(h.administrative_status,'active')=$6)
            AND ($9::boolean OR $7::uuid IS NULL OR EXISTS(SELECT 1 FROM case_memberships m JOIN users u ON u.id=m.user_id
                WHERE m.case_id=c.id AND u.id=$7 AND u.active AND u.role='litigator'))
            ORDER BY c.id LIMIT $8", &[&timestamp(f.period_from)?, &timestamp(f.period_before)?, &timestamp(now)?,
            &(scope == CaseReportScope::Office), &actor.id.as_uuid(), &status,
            &f.litigator.map(UserId::as_uuid), &((MAX_REPORT_CASES + 1) as i64), &(f.kind == CaseReportKind::LitigatorActivity)]).map_err(port)?;
    if rows.len() > MAX_REPORT_CASES {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    Ok(rows.iter().map(|row| row.get(0)).collect())
}
