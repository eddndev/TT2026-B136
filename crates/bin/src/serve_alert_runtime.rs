//! Bounded alert scheduling and delivery outside persistence transactions.

use crate::serve_deadline_runtime::RuntimeControl;
use application::{alerts::*, ApplicationError, PortFailureKind};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AlertRuntimeConfig {
    budget: u32,
    poll: Duration,
}
impl AlertRuntimeConfig {
    pub(crate) fn new(budget: u32, poll: Duration) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&budget) || poll.is_zero() {
            return Err(ApplicationError::InvalidConfiguration(
                "alert processing requires a budget of 1..100 and a positive pause".into(),
            ));
        }
        Ok(Self { budget, poll })
    }
    pub(crate) fn budget(self) -> u32 {
        self.budget
    }
    pub(crate) fn poll(self) -> Duration {
        self.poll
    }
}
impl Default for AlertRuntimeConfig {
    fn default() -> Self {
        Self {
            budget: 20,
            poll: Duration::from_secs(1),
        }
    }
}
pub(crate) fn run(
    scheduler: &dyn AlertSchedulerStore,
    delivery: &dyn AlertDeliveryStore,
    sender: Option<&dyn AlertEmailSender>,
    config: AlertRuntimeConfig,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    let mut pending = None;
    while !control.is_stopped() {
        if pending.is_none() {
            let result = cycle(scheduler, delivery, sender, config, control);
            match result {
                Ok(completion) => pending = completion,
                Err(error) => recover(error)?,
            }
        }
        if let Some(completion) = pending.as_ref() {
            match delivery.complete_attempt(completion.clone()) {
                Ok(()) => pending = None,
                Err(error) => recover(error)?,
            }
        }
        if !control.is_stopped() {
            control.wait(config.poll());
        }
    }
    Ok(())
}
fn cycle(
    scheduler: &dyn AlertSchedulerStore,
    delivery: &dyn AlertDeliveryStore,
    sender: Option<&dyn AlertEmailSender>,
    config: AlertRuntimeConfig,
    control: &dyn RuntimeControl,
) -> Result<Option<AlertDeliveryCompletion>, ApplicationError> {
    for _ in 0..config.budget() {
        if control.is_stopped() {
            return Ok(None);
        }
        if scheduler.run_next()? == AlertSchedulerRun::Idle {
            break;
        }
    }
    if control.is_stopped() {
        return Ok(None);
    }
    let Some(sender) = sender else {
        return Ok(None);
    };
    let Some(claim) = delivery.claim_next()? else {
        return Ok(None);
    };
    if control.is_stopped() {
        return Ok(None);
    }
    let outcome = sender.send(&claim.message);
    // A stop during provider I/O still records the outcome already obtained.
    Ok(Some(AlertDeliveryCompletion {
        delivery_id: claim.delivery_id,
        claim_id: claim.claim_id,
        attempt: claim.attempt,
        outcome,
    }))
}
fn recover(error: ApplicationError) -> Result<(), ApplicationError> {
    let category = match error {
        ApplicationError::Port(_) => "port",
        ApplicationError::ClassifiedPort { kind, .. } => match kind {
            PortFailureKind::Busy => "busy",
            PortFailureKind::Interrupted => "interrupted",
            PortFailureKind::Unavailable => "unavailable",
        },
        error => return Err(error),
    };
    tracing::warn!(category, "alert processing will retry after its pause");
    Ok(())
}
#[cfg(test)]
#[path = "serve_alert_runtime_support.rs"]
mod support;
#[cfg(test)]
#[path = "serve_alert_runtime_tests.rs"]
mod tests;
