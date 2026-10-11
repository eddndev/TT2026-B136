//! Serial, optional ready-report email attempts under coordinated shutdown.
use crate::serve_deadline_runtime::RuntimeControl;
use application::{case_reports::CaseReportEmailRun, ApplicationError};
use std::{sync::Arc, time::Duration};

pub(crate) type ReportEmailConsumer =
    Arc<dyn Fn() -> Result<CaseReportEmailRun, ApplicationError> + Send + Sync>;

pub(crate) fn run(
    next: &dyn Fn() -> Result<CaseReportEmailRun, ApplicationError>,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    while !control.is_stopped() {
        match next() {
            Ok(_) => (),
            Err(ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. }) => {
                tracing::warn!("report email processing will retry after its pause");
            }
            Err(error) => return Err(error),
        }
        if !control.is_stopped() {
            control.wait(Duration::from_secs(1));
        }
    }
    Ok(())
}
