//! Non-durable single-request recovery consumer sharing the HTTP work budget.

use crate::serve_stop::Stop;
use application::{identity::password_reset::ResetRequestAccepted, ApplicationError};
use std::sync::Arc;
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore, TryAcquireError};
use web::{
    password_reset::{PasswordResetRequests, RequestAdmission},
    HttpWorkBudget,
};
use zeroize::Zeroizing;

pub(crate) type ResetRequestWork =
    Arc<dyn Fn(Zeroizing<String>) -> Result<ResetRequestAccepted, ApplicationError> + Send + Sync>;

struct Request {
    email: Zeroizing<String>,
    slot: OwnedSemaphorePermit,
}

#[derive(Clone)]
pub(crate) struct PasswordResetAdmission {
    sender: mpsc::Sender<Request>,
    slot: Arc<Semaphore>,
    stop: Arc<Stop>,
}

impl PasswordResetRequests for PasswordResetAdmission {
    fn try_submit(&self, email: Zeroizing<String>) -> RequestAdmission {
        self.stop
            .while_running(|| {
                if self.sender.is_closed() {
                    return RequestAdmission::Unavailable;
                }
                let slot = match Arc::clone(&self.slot).try_acquire_owned() {
                    Ok(slot) => slot,
                    Err(TryAcquireError::NoPermits) => return RequestAdmission::Busy,
                    Err(TryAcquireError::Closed) => return RequestAdmission::Unavailable,
                };
                match self.sender.try_send(Request { email, slot }) {
                    Ok(()) => RequestAdmission::Accepted,
                    Err(mpsc::error::TrySendError::Full(_)) => RequestAdmission::Busy,
                    Err(mpsc::error::TrySendError::Closed(_)) => RequestAdmission::Unavailable,
                }
            })
            .unwrap_or(RequestAdmission::Unavailable)
    }
}

pub(crate) struct PasswordResetConsumer {
    requests: mpsc::Receiver<Request>,
    work: ResetRequestWork,
    budget: HttpWorkBudget,
    stop: Arc<Stop>,
}

impl PasswordResetConsumer {
    /// The caller retains synchronous adapter owners outside its async runtime.
    pub(crate) fn new(
        work: ResetRequestWork,
        budget: HttpWorkBudget,
        stop: Arc<Stop>,
    ) -> (PasswordResetAdmission, Self) {
        let (sender, requests) = mpsc::channel(1);
        let admission = PasswordResetAdmission {
            sender,
            slot: Arc::new(Semaphore::new(1)),
            stop: Arc::clone(&stop),
        };
        (
            admission,
            Self {
                requests,
                work,
                budget,
                stop,
            },
        )
    }

    pub(crate) async fn run(mut self) -> Result<(), ApplicationError> {
        loop {
            let request = tokio::select! {
                biased;
                () = self.stop.stopped() => return Ok(()),
                request = self.requests.recv() => match request {
                    Some(request) => request,
                    None => return self.failed(),
                },
            };
            let permit = tokio::select! {
                biased;
                () = self.stop.stopped() => return Ok(()),
                permit = self.budget.acquire_owned() => match permit {
                    Ok(permit) => permit,
                    Err(_) => return self.failed(),
                },
            };
            let work = Arc::clone(&self.work);
            let task = self.stop.while_running(move || {
                tokio::task::spawn_blocking(move || {
                    // Both permits belong to actual work, never its waiting future.
                    let _budget_permit = permit;
                    let _request_permit = request.slot;
                    work(request.email)
                })
            });
            let Some(task) = task else { return Ok(()) };
            // Stop cannot cancel a synchronous driver; always retain its join.
            match task.await {
                Ok(Ok(_)) => {}
                Ok(Err(_)) => tracing::warn!(
                    code = "password_reset_request_failed",
                    "password recovery request ended"
                ),
                Err(_) => return self.failed(),
            }
        }
    }

    fn failed(&self) -> Result<(), ApplicationError> {
        self.stop.request();
        Err(ApplicationError::Port(
            "password reset consumer unavailable".into(),
        ))
    }
}
