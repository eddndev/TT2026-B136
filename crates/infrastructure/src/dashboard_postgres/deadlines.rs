use super::{capacity, inconsistent, port, MAX_DEADLINES};
use application::{
    dashboard::DashboardSnapshot,
    deadlines::{DeadlineAttention, DeadlineId, DeadlineStatus},
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;
use time::Duration;
use uuid::Uuid;

pub(super) fn count(
    tx: &mut Transaction<'_>,
    cases: &[Uuid],
    hasher: &dyn DocumentHasher,
    snapshot: &mut DashboardSnapshot,
) -> Result<(), ApplicationError> {
    let rows = tx.query("SELECT case_id,id FROM case_deadlines WHERE case_id=ANY($1::uuid[]) ORDER BY case_id,id LIMIT $2", &[&cases, &((MAX_DEADLINES + 1) as i64)]).map_err(port)?;
    capacity(rows.len(), MAX_DEADLINES, "deadline")?;
    let at = snapshot.checked_at;
    let hours = at
        .checked_add(Duration::hours(48))
        .ok_or_else(|| inconsistent("48 hour boundary is outside supported time"))?;
    let week = at
        .checked_add(Duration::days(7))
        .ok_or_else(|| inconsistent("seven day boundary is outside supported time"))?;
    for row in rows {
        let case = CaseId::from_uuid(row.try_get("case_id").map_err(port)?);
        let id = DeadlineId::from_uuid(row.try_get("id").map_err(port)?);
        let detail = crate::deadline_postgres::storage::detail(tx, case, id, None, hasher)?;
        if detail.status == DeadlineStatus::Retired
            || matches!(detail.attention, DeadlineAttention::Recorded { .. })
        {
            continue;
        }
        let current = crate::deadline_postgres::current_in_transaction(tx, &detail, hasher, at)?;
        match current.operational().due_at() {
            Some(due) if due < at => snapshot.deadlines_overdue += 1,
            Some(due) => {
                if due < hours {
                    snapshot.deadlines_due_48h += 1;
                }
                if due < week {
                    snapshot.deadlines_due_7d += 1;
                }
            }
            None => snapshot.deadlines_unresolved += 1,
        }
    }
    Ok(())
}
