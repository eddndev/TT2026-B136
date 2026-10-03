//! Shared stop request and interruptible consumer pause.

use crate::serve_deadline_runtime::RuntimeControl;
use std::{
    sync::{Condvar, Mutex},
    time::Duration,
};

pub(crate) struct Stop {
    stopped: Mutex<bool>,
    wake: Condvar,
    asynchronous: tokio::sync::watch::Sender<bool>,
}

impl Default for Stop {
    fn default() -> Self {
        let (asynchronous, _) = tokio::sync::watch::channel(false);
        Self {
            stopped: Mutex::new(false),
            wake: Condvar::new(),
            asynchronous,
        }
    }
}

impl Stop {
    pub(crate) fn request(&self) {
        let mut stopped = self
            .stopped
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *stopped = true;
        self.asynchronous.send_replace(true);
        self.wake.notify_all();
    }

    /// Shares the stop linearization point with short, nonblocking admission work.
    pub(crate) fn while_running<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        let stopped = self
            .stopped
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *stopped {
            None
        } else {
            Some(action())
        }
    }

    /// A subscriber created after the stop request also observes it immediately.
    pub(crate) async fn stopped(&self) {
        let mut receiver = self.asynchronous.subscribe();
        let _ = receiver.wait_for(|stopped| *stopped).await;
    }
}

impl RuntimeControl for Stop {
    fn is_stopped(&self) -> bool {
        *self
            .stopped
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn wait(&self, duration: Duration) {
        let stopped = self
            .stopped
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _result = self
            .wake
            .wait_timeout_while(stopped, duration, |value| !*value);
    }
}

#[cfg(test)]
#[path = "serve_stop_tests.rs"]
mod tests;
