use super::{capacity, port, MAX_WORKLOAD};
use application::{dashboard::DashboardWorkload, identity::Principal, ApplicationError};
use domain::identity::{Role, UserId};
use postgres::Transaction;
use uuid::Uuid;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    active_cases: &[Uuid],
) -> Result<Vec<DashboardWorkload>, ApplicationError> {
    let rows = tx.query(
        "SELECT u.id,u.email,
            (SELECT count(*) FROM case_memberships m WHERE m.user_id=u.id AND m.case_id=ANY($1::uuid[])) AS active_cases
         FROM users u WHERE u.active AND u.role='litigator'
            AND ($2::boolean OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.user_id=u.id AND m.case_id=ANY($1::uuid[])))
         ORDER BY u.id LIMIT $3 FOR SHARE OF u",
        &[&active_cases, &(actor.role == Role::Owner), &((MAX_WORKLOAD + 1) as i64)],
    ).map_err(port)?;
    capacity(rows.len(), MAX_WORKLOAD, "workload")?;
    rows.iter()
        .map(|row| {
            Ok(DashboardWorkload {
                user_id: UserId::from_uuid(row.try_get("id").map_err(port)?),
                email: row.try_get("email").map_err(port)?,
                active_cases: u64::try_from(row.try_get::<_, i64>("active_cases").map_err(port)?)
                    .map_err(super::inconsistent)?,
            })
        })
        .collect()
}
