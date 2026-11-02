//! Join every background consumer after any exit or requested stop.

use crate::{serve_deadline_runtime::RuntimeControl, serve_stop::Stop};
use application::ApplicationError;
use std::sync::Arc;
use tokio::task::{JoinError, JoinHandle};

type ConsumerResult = Result<Result<(), ApplicationError>, JoinError>;
type Task = JoinHandle<Result<(), ApplicationError>>;

/// The consumer whose exit ends supervision, in the order failures are
/// reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    Deadlines,
    Alerts,
    Reports,
    ReportEmail,
    Invitations,
    PasswordReset,
    Contracts,
}

impl Exit {
    /// The stable codes of an unexpected stop, a failure and a panic.
    const fn codes(self) -> [&'static str; 3] {
        match self {
            Self::Deadlines => ["deadline_stopped", "deadline_failed", "deadline_panicked"],
            Self::Alerts => ["alerts_stopped", "alerts_failed", "alerts_panicked"],
            Self::Reports => ["reports_stopped", "reports_failed", "reports_panicked"],
            Self::ReportEmail => [
                "report_email_stopped",
                "report_email_failed",
                "report_email_panicked",
            ],
            Self::Invitations => [
                "invitations_stopped",
                "invitations_failed",
                "invitations_panicked",
            ],
            Self::PasswordReset => [
                "password_reset_stopped",
                "password_reset_failed",
                "password_reset_panicked",
            ],
            Self::Contracts => [
                "contracts_stopped",
                "contracts_failed",
                "contracts_panicked",
            ],
        }
    }
}

#[cfg(test)]
pub(crate) async fn supervise(
    deadlines: Task,
    alerts: Task,
    reports: Task,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    supervise_with_password_reset(deadlines, alerts, reports, None, stop).await
}

#[cfg(test)]
pub(crate) async fn supervise_with_password_reset(
    deadlines: Task,
    alerts: Task,
    reports: Task,
    password_reset: Option<Task>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    supervise_with_invitations(deadlines, alerts, reports, password_reset, None, stop).await
}

#[cfg(test)]
pub(crate) async fn supervise_with_invitations(
    deadlines: Task,
    alerts: Task,
    reports: Task,
    password_reset: Option<Task>,
    invitations: Option<Task>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    supervise_with_report_email(
        deadlines,
        alerts,
        reports,
        password_reset,
        invitations,
        None,
        stop,
    )
    .await
}

#[cfg(test)]
pub(crate) async fn supervise_with_report_email(
    deadlines: Task,
    alerts: Task,
    reports: Task,
    password_reset: Option<Task>,
    invitations: Option<Task>,
    report_email: Option<Task>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    supervise_with_contracts(
        deadlines,
        alerts,
        reports,
        password_reset,
        invitations,
        report_email,
        None,
        stop,
    )
    .await
}

/// Waits for the first consumer exit, requests stop, joins every other
/// consumer and reports each unexpected stop, failure or panic by code.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn supervise_with_contracts(
    deadlines: Task,
    alerts: Task,
    reports: Task,
    password_reset: Option<Task>,
    invitations: Option<Task>,
    report_email: Option<Task>,
    contracts: Option<Task>,
    stop: Arc<Stop>,
) -> Result<(), ApplicationError> {
    let mut tasks = [
        (Exit::Deadlines, Some(deadlines)),
        (Exit::Alerts, Some(alerts)),
        (Exit::Reports, Some(reports)),
        (Exit::ReportEmail, report_email),
        (Exit::Invitations, invitations),
        (Exit::PasswordReset, password_reset),
        (Exit::Contracts, contracts),
    ];
    let (exit, first) = first_exit(&mut tasks).await;
    let was_stopped = stop.is_stopped();
    stop.request();
    let mut failures = Vec::new();
    if !was_stopped && matches!(first, Ok(Ok(()))) {
        failures.push(exit.codes()[0]);
    }
    let mut first = Some(first);
    for (kind, task) in tasks {
        let result = match task {
            Some(task) => task.await,
            None if kind == exit => first.take().unwrap_or(Ok(Ok(()))),
            None => Ok(Ok(())),
        };
        match result {
            Ok(Ok(())) => (),
            Ok(Err(_)) => failures.push(kind.codes()[1]),
            Err(_) => failures.push(kind.codes()[2]),
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

/// The first consumer to exit; its slot is emptied because a finished
/// handle cannot be awaited again.
async fn first_exit(tasks: &mut [(Exit, Option<Task>); 7]) -> (Exit, ConsumerResult) {
    let (exit, result) = {
        let [a, b, c, d, e, f, g] = &mut *tasks;
        tokio::select! {
            result = optional(&mut a.1) => (a.0, result),
            result = optional(&mut b.1) => (b.0, result),
            result = optional(&mut c.1) => (c.0, result),
            result = optional(&mut d.1) => (d.0, result),
            result = optional(&mut e.1) => (e.0, result),
            result = optional(&mut f.1) => (f.0, result),
            result = optional(&mut g.1) => (g.0, result),
        }
    };
    for (kind, task) in tasks.iter_mut() {
        if *kind == exit {
            *task = None;
        }
    }
    (exit, result)
}

async fn optional(task: &mut Option<Task>) -> ConsumerResult {
    match task {
        Some(task) => task.await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
#[path = "serve_report_supervisor_tests.rs"]
mod report_tests;
#[cfg(test)]
#[path = "serve_alert_supervisor_tests.rs"]
mod tests;
