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
    PasswordReset(ConsumerResult),
}
pub(crate) async fn supervise(
    deadlines: JoinHandle<Result<(), ApplicationError>>,
    alerts: JoinHandle<Result<(), ApplicationError>>,
    reports: JoinHandle<Result<(), ApplicationError>>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    supervise_with_password_reset(deadlines, alerts, reports, None, stop).await
}

pub(crate) async fn supervise_with_password_reset(
    mut deadlines: JoinHandle<Result<(), ApplicationError>>,
    mut alerts: JoinHandle<Result<(), ApplicationError>>,
    mut reports: JoinHandle<Result<(), ApplicationError>>,
    mut password_reset: Option<JoinHandle<Result<(), ApplicationError>>>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    let exit = {
        let reset_exit = async {
            match &mut password_reset {
                Some(task) => task.await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            result = &mut deadlines => Exit::Deadlines(result),
            result = &mut alerts => Exit::Alerts(result),
            result = &mut reports => Exit::Reports(result),
            result = reset_exit => Exit::PasswordReset(result),
        }
    };
    let was_stopped = stop.is_stopped();
    stop.request();
    let mut failures = Vec::new();
    let (deadline_result, alert_result, report_result, reset_result) = match exit {
        Exit::Deadlines(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("deadline_stopped");
            }
            let (alert_result, report_result, reset_result) =
                tokio::join!(alerts, reports, join_optional(password_reset));
            (result, alert_result, report_result, reset_result)
        }
        Exit::Alerts(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("alerts_stopped");
            }
            let (deadline_result, report_result, reset_result) =
                tokio::join!(deadlines, reports, join_optional(password_reset));
            (deadline_result, result, report_result, reset_result)
        }
        Exit::Reports(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("reports_stopped");
            }
            let (deadline_result, alert_result, reset_result) =
                tokio::join!(deadlines, alerts, join_optional(password_reset));
            (deadline_result, alert_result, result, reset_result)
        }
        Exit::PasswordReset(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("password_reset_stopped");
            }
            let (deadline_result, alert_result, report_result) =
                tokio::join!(deadlines, alerts, reports);
            (deadline_result, alert_result, report_result, result)
        }
    };
    for (result, failed, panicked) in [
        (deadline_result, "deadline_failed", "deadline_panicked"),
        (alert_result, "alerts_failed", "alerts_panicked"),
        (report_result, "reports_failed", "reports_panicked"),
        (
            reset_result,
            "password_reset_failed",
            "password_reset_panicked",
        ),
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

async fn join_optional(task: Option<JoinHandle<Result<(), ApplicationError>>>) -> ConsumerResult {
    match task {
        Some(task) => task.await,
        None => Ok(Ok(())),
    }
}
#[cfg(test)]
#[path = "serve_report_supervisor_tests.rs"]
mod report_tests;
#[cfg(test)]
#[path = "serve_alert_supervisor_tests.rs"]
mod tests;
