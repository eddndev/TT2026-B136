use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::Notify;

#[derive(Default)]
struct State {
    pending: AtomicUsize,
    changed: Notify,
}

/// Wait for admitted operations after the HTTP listener has stopped and drained.
#[derive(Clone, Default)]
pub struct AccessAuditDrain(Arc<State>);

impl AccessAuditDrain {
    pub async fn drain(&self) {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.0.pending.load(Ordering::Acquire) == 0 {
                return;
            }
            changed.await;
        }
    }

    pub(super) fn reserve(&self) -> Admission {
        self.0.pending.fetch_add(1, Ordering::AcqRel);
        Admission(self.0.clone())
    }
}

pub(crate) struct Admission(Arc<State>);
impl Drop for Admission {
    fn drop(&mut self) {
        if self.0.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.changed.notify_waiters();
        }
    }
}
