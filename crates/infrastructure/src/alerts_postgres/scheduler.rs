use super::{
    audit, codec, port, schedule_activate, schedule_scan, schedule_selection, PostgresAlertStore,
};
use application::{alerts::*, ApplicationError};

impl AlertSchedulerStore for PostgresAlertStore {
    fn run_next(&self) -> Result<AlertSchedulerRun, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let now = self.now()?;
        let result = if let Some(result) = schedule_activate::next(self, &mut tx, now)? {
            result
        } else if let Some(selection) = schedule_selection::next(&mut tx, now)? {
            schedule_scan::run(self, &mut tx, selection, now)?
        } else {
            tx.commit().map_err(port)?;
            return Ok(AlertSchedulerRun::Idle);
        };
        let (action, resource) = match &result {
            AlertSchedulerRun::Activated { alert_id } => {
                ("alert.activated", format!("alert:{alert_id}"))
            }
            AlertSchedulerRun::Reconciled {
                subject,
                scheduled,
                superseded,
            } => {
                let (kind, id) = codec::subject_key(*subject);
                (
                    "alert.reconciled",
                    format!("subject:{kind}:{id}:scheduled:{scheduled}:superseded:{superseded}"),
                )
            }
            AlertSchedulerRun::Idle => unreachable!("non-idle scheduler outcome required"),
        };
        audit(&mut tx, "system:activity-alerts", action, &resource, now)?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
