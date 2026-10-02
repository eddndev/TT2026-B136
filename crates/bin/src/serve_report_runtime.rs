//! Serial report jobs with bounded pauses and coordinated shutdown.
use crate::serve_deadline_runtime::RuntimeControl;
use application::{
    case_reports::{CaseReportError, CaseReportWorkerRun},
    ApplicationError,
};
use std::time::Duration;

pub(crate) fn run(
    next: &dyn Fn() -> Result<CaseReportWorkerRun, ApplicationError>,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    while !control.is_stopped() {
        match next() {
            Ok(_) => (),
            Err(ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. }) => {
                tracing::warn!("report processing will retry after its pause");
            }
            Err(ApplicationError::CaseReport(CaseReportError::LeaseLost)) => {
                tracing::warn!("report processing lost its lease; the durable queue owns recovery");
            }
            Err(error) => return Err(error),
        }
        if !control.is_stopped() {
            control.wait(Duration::from_secs(1));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "serve_report_runtime_tests.rs"]
mod tests;
