use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use crate::support::{binding, principal, Harness, MockStore, State};
use application::identity::owner_certificates::OwnerCertificateService;

pub struct Gate {
    entered: tokio::sync::oneshot::Receiver<()>,
    release: Option<mpsc::Sender<()>>,
}

impl Gate {
    pub async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), &mut self.entered)
            .await
            .expect("Owner worker did not reach the controlled evidence port")
            .unwrap();
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

pub fn blocked() -> (Arc<OwnerCertificateService>, Arc<Mutex<State>>, Gate) {
    let mut harness = Harness::new();
    let state = harness.state.clone();
    let observed = state.clone();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    harness.store = MockStore::new();
    harness
        .store
        .expect_find()
        .times(1)
        .return_once(move |actor, id| {
            assert_eq!((actor, id), (principal().id, binding()));
            observed.lock().unwrap().calls.push("find");
            let _ = entered.send(());
            wait.recv_timeout(Duration::from_secs(4))
                .expect("Owner evidence gate was not released");
            Ok(None)
        });
    (
        Arc::new(harness.service()),
        state,
        Gate {
            entered: ready,
            release: Some(release),
        },
    )
}
