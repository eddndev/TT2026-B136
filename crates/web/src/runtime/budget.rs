//! Owned blocking-work capacity shared by HTTP and supervised consumers.

use std::{num::NonZeroUsize, sync::Arc};

use application::ApplicationError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Clones share the same fixed blocking-work capacity.
#[derive(Clone)]
pub struct HttpWorkBudget {
    slots: Arc<Semaphore>,
    capacity: NonZeroUsize,
}

/// Hold this permit inside the actual blocking operation until it finishes.
/// Dropping the waiting HTTP or consumer future must not release running work.
#[must_use = "the permit must be held until the blocking operation finishes"]
pub struct HttpWorkPermit {
    _permit: OwnedSemaphorePermit,
}

impl HttpWorkBudget {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(capacity.get())),
            capacity,
        }
    }

    /// Original capacity, including permits currently held by running work.
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    pub async fn acquire_owned(&self) -> Result<HttpWorkPermit, ApplicationError> {
        let permit = self
            .slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ApplicationError::Port("HTTP work budget unavailable".into()))?;
        Ok(HttpWorkPermit { _permit: permit })
    }

    #[cfg(test)]
    pub(super) fn available_permits(&self) -> usize {
        self.slots.available_permits()
    }
}
