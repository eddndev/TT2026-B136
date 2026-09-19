//! Shared stop request and interruptible consumer pause.

use crate::serve_deadline_runtime::RuntimeControl;
use std::{
    sync::{Condvar, Mutex},
    time::Duration,
};

#[derive(Default)]
pub(crate) struct Stop {
    stopped: Mutex<bool>,
    wake: Condvar,
}

impl Stop {
    pub(crate) fn request(&self) {
        let mut stopped = self
            .stopped
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *stopped = true;
        self.wake.notify_all();
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
