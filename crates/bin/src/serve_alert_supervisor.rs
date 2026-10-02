//! Join every background consumer after any exit or requested stop.

use crate::{serve_deadline_runtime::RuntimeControl, serve_stop::Stop};
use application::ApplicationError;
use std::sync::Arc;
use tokio::task::{JoinError, JoinHandle};

type ConsumerResult = Result<Result<(), ApplicationError>, JoinError>;
enum Exit {
    Deadlines(ConsumerResult),
    Alerts(ConsumerResult),
    Reports(ConsumerResult),
}
pub(crate) async fn supervise(
    mut deadlines: JoinHandle<Result<(), ApplicationError>>,
    mut alerts: JoinHandle<Result<(), ApplicationError>>,
    mut reports: JoinHandle<Result<(), ApplicationError>>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    let exit = tokio::select! {
        result = &mut deadlines => Exit::Deadlines(result),
        result = &mut alerts => Exit::Alerts(result),
        result = &mut reports => Exit::Reports(result),
    };
    let was_stopped = stop.is_stopped();
    stop.request();
    let mut failures = Vec::new();
    let (deadline_result, alert_result, report_result) = match exit {
        Exit::Deadlines(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("deadline_stopped");
            }
            let (alert_result, report_result) = tokio::join!(alerts, reports);
            (result, alert_result, report_result)
        }
        Exit::Alerts(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("alerts_stopped");
            }
            let (deadline_result, report_result) = tokio::join!(deadlines, reports);
            (deadline_result, result, report_result)
        }
        Exit::Reports(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("reports_stopped");
            }
            let (deadline_result, alert_result) = tokio::join!(deadlines, alerts);
            (deadline_result, alert_result, result)
        }
    };
    for (result, failed, panicked) in [
        (deadline_result, "deadline_failed", "deadline_panicked"),
        (alert_result, "alerts_failed", "alerts_panicked"),
        (report_result, "reports_failed", "reports_panicked"),
    ] {
        match result {
            Ok(Ok(())) => (),
            Ok(Err(_)) => failures.push(failed),
            Err(_) => failures.push(panicked),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(ApplicationError::Port(format!(
            "activity consumers stopped: {}",
            failures.join(",")
        )))
    }
}
#[cfg(test)]
#[path = "serve_report_supervisor_tests.rs"]
mod report_tests;
#[cfg(test)]
#[path = "serve_alert_supervisor_tests.rs"]
mod tests;
