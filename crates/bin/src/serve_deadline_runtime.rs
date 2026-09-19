//! Serial scheduling of durable deadline dispatch and execution.

use application::{
    deadline_dispatch::{
        DeadlineDispatchLimit, DeadlineDispatchRequest, DeadlineDispatchStore,
        DeadlineDispatchStream,
    },
    deadline_worker::{DeadlineWorkerRun, DeadlineWorkerStore},
    ApplicationError, PortFailureKind,
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeadlineRuntimeConfig {
    limit: DeadlineDispatchLimit,
    poll: Duration,
}

impl DeadlineRuntimeConfig {
    pub(crate) fn new(
        limit: DeadlineDispatchLimit,
        poll: Duration,
    ) -> Result<Self, ApplicationError> {
        if poll.is_zero() {
            return Err(ApplicationError::InvalidConfiguration(
                "deadline processing requires a positive pause".into(),
            ));
        }
        Ok(Self { limit, poll })
    }

    pub(crate) fn limit(self) -> DeadlineDispatchLimit {
        self.limit
    }

    pub(crate) fn poll(self) -> Duration {
        self.poll
    }
}

pub(crate) trait RuntimeControl: Send + Sync {
    fn is_stopped(&self) -> bool;
    fn wait(&self, duration: Duration);
}

pub(crate) fn run(
    dispatch: &dyn DeadlineDispatchStore,
    worker: &dyn DeadlineWorkerStore,
    config: DeadlineRuntimeConfig,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    let mut stream = DeadlineDispatchStream::Events;
    while !control.is_stopped() {
        if let Err(error) = dispatch.dispatch(DeadlineDispatchRequest {
            stream,
            limit: config.limit(),
        }) {
            recover(error, "dispatch")?;
        }
        for _ in 0..config.limit().get() {
            if control.is_stopped() {
                break;
            }
            match worker.run_next() {
                Ok(DeadlineWorkerRun::Idle) => break,
                Ok(DeadlineWorkerRun::Completed(_) | DeadlineWorkerRun::Deferred(_)) => {}
                Err(error) => {
                    recover(error, "worker")?;
                    break;
                }
            }
        }
        if !control.is_stopped() {
            control.wait(config.poll());
        }
        stream = match stream {
            DeadlineDispatchStream::Events => DeadlineDispatchStream::LegacyBootstrap,
            DeadlineDispatchStream::LegacyBootstrap => DeadlineDispatchStream::Events,
        };
    }
    Ok(())
}

fn recover(error: ApplicationError, operation: &'static str) -> Result<(), ApplicationError> {
    let category = match error {
        ApplicationError::Port(_) => "port",
        ApplicationError::ClassifiedPort { kind, .. } => match kind {
            PortFailureKind::Busy => "busy",
            PortFailureKind::Interrupted => "interrupted",
            PortFailureKind::Unavailable => "unavailable",
        },
        error => return Err(error),
    };
    tracing::warn!(
        operation,
        category,
        "deadline processing will retry after its pause"
    );
    Ok(())
}

#[cfg(test)]
#[path = "serve_deadline_runtime_concurrency_tests.rs"]
mod concurrency_tests;
#[cfg(test)]
#[path = "serve_deadline_runtime_support.rs"]
mod support;
#[cfg(test)]
#[path = "serve_deadline_runtime_tests.rs"]
mod tests;
