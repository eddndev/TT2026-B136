//! The supervised contract consumer: every cycle closes exhausted requests
//! and, only when contract e-mail is enabled, sends up to one batch of
//! notices. A transient storage or provider failure waits for the next
//! cycle; any other failure ends the consumer for supervision. See
//! docs/adr/0086-bilateral-contract-signing.md.
use crate::serve_deadline_runtime::RuntimeControl;
use application::{
    contracts::{ContractNoticeRun, NOTICE_BATCH},
    ApplicationError,
};
use std::{sync::Arc, time::Duration};

/// The pause between two cycles.
pub(crate) const CONTRACT_POLL: Duration = Duration::from_secs(5);

pub(crate) type ExpiryCycle = Arc<dyn Fn() -> Result<usize, ApplicationError> + Send + Sync>;
pub(crate) type NoticeAttempt =
    Arc<dyn Fn() -> Result<ContractNoticeRun, ApplicationError> + Send + Sync>;

#[derive(Clone)]
pub(crate) struct ContractConsumer {
    pub expiry: ExpiryCycle,
    pub notices: Option<NoticeAttempt>,
}

#[cfg(test)]
impl ContractConsumer {
    /// A consumer with nothing to close and no e-mail.
    pub(crate) fn idle() -> Self {
        Self {
            expiry: Arc::new(|| Ok(0)),
            notices: None,
        }
    }
}

pub(crate) fn run(
    consumer: &ContractConsumer,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    while !control.is_stopped() {
        if let Err(error) = (consumer.expiry)() {
            transient(error, "contract expiry will retry after its pause")?;
        }
        if let Some(notices) = &consumer.notices {
            for _ in 0..NOTICE_BATCH {
                if control.is_stopped() {
                    break;
                }
                match notices() {
                    Ok(ContractNoticeRun::Idle) => break,
                    Ok(_) => (),
                    Err(error) => {
                        transient(error, "contract notice delivery will retry after its pause")?;
                        break;
                    }
                }
            }
        }
        if !control.is_stopped() {
            control.wait(CONTRACT_POLL);
        }
    }
    Ok(())
}

fn transient(error: ApplicationError, message: &'static str) -> Result<(), ApplicationError> {
    match error {
        ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. } => {
            tracing::warn!("{message}");
            Ok(())
        }
        error => Err(error),
    }
}

#[cfg(test)]
#[path = "serve_contract_runtime_tests.rs"]
mod tests;
