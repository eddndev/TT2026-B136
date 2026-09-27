use super::{cases, deadlines, documents, inconsistent, port, workload, PostgresDashboardStore};
use application::{dashboard::*, ApplicationError};
use domain::identity::{Role, UserId};
use time::UtcOffset;

impl DashboardStore for PostgresDashboardStore {
    fn read(&self, actor: UserId) -> Result<DashboardSnapshot, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| inconsistent("database lock poisoned"))?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = crate::postgres_actor::active_actor(&mut tx, actor)?;
        let scope = match principal.role {
            Role::Owner => DashboardScope::Office,
            Role::Litigator => DashboardScope::AssignedCases,
            _ => return Err(ApplicationError::PermissionDenied),
        };
        let checked_at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&checked_at.year()) {
            return Err(inconsistent("clock is outside supported years"));
        }
        let cases = cases::load(&mut tx, &principal, self.hasher.as_ref())?;
        let mut snapshot = DashboardSnapshot {
            scope,
            checked_at,
            active_cases: cases.active.len() as u64,
            pending_contracts: documents::pending_contracts(&mut tx, &cases.all)?,
            deadlines_overdue: 0,
            deadlines_due_48h: 0,
            deadlines_due_7d: 0,
            deadlines_unresolved: 0,
            workload: workload::load(&mut tx, &principal, &cases.active)?,
        };
        deadlines::count(&mut tx, &cases.all, self.hasher.as_ref(), &mut snapshot)?;
        snapshot.validate(scope)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "dashboard.read",
            &format!(
                "dashboard:scope:{}:checked_at:{}",
                scope.as_str(),
                checked_at.unix_timestamp()
            ),
            checked_at,
        )?;
        tx.commit().map_err(port)?;
        Ok(snapshot)
    }
}
