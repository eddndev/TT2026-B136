use crate::password_reset_http_support::Reply;
use application::{
    dashboard::{DashboardSnapshot, DashboardWorkflow},
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Mutex,
};
use std::time::Duration;
use tower::ServiceExt;

#[derive(Default)]
pub struct Dashboard {
    calls: AtomicUsize,
    wait: Mutex<Option<Wait>>,
}
struct Wait {
    entered: tokio::sync::oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}
pub struct Gate {
    entered: tokio::sync::oneshot::Receiver<()>,
    release: Option<mpsc::Sender<()>>,
}
impl Gate {
    pub async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), &mut self.entered)
            .await
            .expect("dashboard did not enter its blocking port")
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
impl Dashboard {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    pub fn block(&self) -> Gate {
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        *self.wait.lock().unwrap() = Some(Wait {
            entered,
            release: wait,
        });
        Gate {
            entered: ready,
            release: Some(release),
        }
    }
}
impl DashboardWorkflow for Dashboard {
    fn read(&self, token: &str) -> Result<DashboardSnapshot, ApplicationError> {
        assert_eq!(token, "dashboard-session");
        self.calls.fetch_add(1, Ordering::SeqCst);
        let wait = self.wait.lock().unwrap().take();
        if let Some(wait) = wait {
            let _ = wait.entered.send(());
            wait.release
                .recv_timeout(Duration::from_secs(4))
                .expect("dashboard blocking gate was not released");
        }
        Err(ApplicationError::PermissionDenied)
    }
}

pub async fn get(router: &Router, path: &str) -> Reply {
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        router.clone().oneshot(
            Request::get(path)
                .header("authorization", "Bearer dashboard-session")
                .body(Body::empty())
                .unwrap(),
        ),
    )
    .await
    .expect("composed HTTP did not finish within the test bound")
    .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap()
        .to_vec();
    Reply {
        status,
        headers,
        body,
    }
}
