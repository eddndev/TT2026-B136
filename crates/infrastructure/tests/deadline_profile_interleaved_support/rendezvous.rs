use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Default)]
struct State {
    arrived: u8,
    failed: bool,
}

/// A single-use rendezvous after both stores release their preparation transactions.
pub struct PrepareRendezvous {
    state: Mutex<State>,
    changed: Condvar,
    timeout: Duration,
}
impl PrepareRendezvous {
    pub fn new(timeout: Duration) -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
            timeout,
        }
    }

    pub fn wait(&self) {
        assert!(
            self.arrive(),
            "prepared peer did not arrive before the rendezvous failed"
        );
    }

    fn arrive(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.failed || state.arrived == 2 {
            return false;
        }
        state.arrived += 1;
        if state.arrived == 2 {
            self.changed.notify_all();
            return true;
        }
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, self.timeout, |state| {
                state.arrived < 2 && !state.failed
            })
            .unwrap();
        if state.arrived == 2 && !state.failed {
            return true;
        }
        // A late participant cannot turn an already failed rendezvous into success.
        state.failed = true;
        self.changed.notify_all();
        false
    }
}
