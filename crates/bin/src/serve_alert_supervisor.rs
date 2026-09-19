//! Join both activity consumers when either terminates or stop is requested.

use crate::{serve_deadline_runtime::RuntimeControl, serve_stop::Stop};
use application::ApplicationError;
use std::sync::Arc;
use tokio::task::{JoinError, JoinHandle};

type ConsumerResult = Result<Result<(), ApplicationError>, JoinError>;
enum Exit {
    Deadlines(ConsumerResult),
    Alerts(ConsumerResult),
}
pub(crate) async fn supervise(
    mut deadlines: JoinHandle<Result<(), ApplicationError>>,
    mut alerts: JoinHandle<Result<(), ApplicationError>>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    let exit = tokio::select! {
        result = &mut deadlines => Exit::Deadlines(result),
        result = &mut alerts => Exit::Alerts(result),
    };
    let was_stopped = stop.is_stopped();
    stop.request();
    let mut failures = Vec::new();
    let (deadline_result, alert_result) = match exit {
        Exit::Deadlines(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("deadline_stopped");
            }
            (result, alerts.await)
        }
        Exit::Alerts(result) => {
            if !was_stopped && matches!(result, Ok(Ok(()))) {
                failures.push("alerts_stopped");
            }
            (deadlines.await, result)
        }
    };
    for (result, failed, panicked) in [
        (deadline_result, "deadline_failed", "deadline_panicked"),
        (alert_result, "alerts_failed", "alerts_panicked"),
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
#[path = "serve_alert_supervisor_tests.rs"]
mod tests;
