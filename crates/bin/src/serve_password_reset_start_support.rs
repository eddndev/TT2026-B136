use crate::{
    serve_alert_composition::AlertConsumers,
    serve_alert_runtime::AlertRuntimeConfig,
    serve_deadline_runtime::DeadlineRuntimeConfig,
    serve_password_reset_runtime::{PasswordResetConsumer, ResetRequestWork},
    serve_start::{PasswordResetRuntime, ServerComponents},
    serve_stop::Stop,
};
use application::{alerts::*, deadline_dispatch::*, deadline_worker::*, ApplicationError};
use axum::{routing::get, Extension, Router};
use domain::typed_participants::Uuid;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        mpsc, Arc, Mutex,
    },
    thread::{self, ThreadId},
    time::Duration,
};
use web::{
    password_reset::{PasswordResetRequests, RequestAdmission},
    HttpWorkBudget,
};
use zeroize::Zeroizing;

pub const WATCHDOG: Duration = Duration::from_secs(5);

pub struct Dropped {
    pub label: &'static str,
    pub in_runtime: bool,
    pub thread: ThreadId,
}
pub type Drops = Arc<Mutex<Vec<Dropped>>>;

pub struct Probe {
    label: &'static str,
    drops: Drops,
}
impl Probe {
    pub fn new(label: &'static str, drops: &Drops) -> Self {
        Self {
            label,
            drops: Arc::clone(drops),
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.lock().unwrap().push(Dropped {
            label: self.label,
            in_runtime: tokio::runtime::Handle::try_current().is_ok(),
            thread: thread::current().id(),
        });
    }
}

pub struct Dispatcher {
    pub fail: Mutex<mpsc::Receiver<()>>,
}
impl DeadlineDispatchStore for Dispatcher {
    fn dispatch(
        &self,
        _: DeadlineDispatchRequest,
    ) -> Result<DeadlineDispatchBatch, ApplicationError> {
        self.fail
            .lock()
            .unwrap()
            .recv_timeout(WATCHDOG * 2)
            .expect("dispatcher gate watchdog");
        Err(ApplicationError::InvalidConfiguration(
            "fixture dispatcher failure".into(),
        ))
    }
}
pub struct Worker;
impl DeadlineWorkerStore for Worker {
    fn run_next(&self) -> Result<DeadlineWorkerRun, ApplicationError> {
        Ok(DeadlineWorkerRun::Idle)
    }
    fn result(&self, _: Uuid) -> Result<Option<DeadlineWorkerResult>, ApplicationError> {
        panic!("no individual result query during server start")
    }
    fn latest_attempt(&self, _: Uuid) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
        panic!("no individual attempt query during server start")
    }
}
struct Alerts;
impl AlertSchedulerStore for Alerts {
    fn run_next(&self) -> Result<AlertSchedulerRun, ApplicationError> {
        Ok(AlertSchedulerRun::Idle)
    }
}
impl AlertDeliveryStore for Alerts {
    fn claim_next(&self) -> Result<Option<AlertDeliveryClaim>, ApplicationError> {
        Ok(None)
    }
    fn complete_attempt(&self, _: AlertDeliveryCompletion) -> Result<(), ApplicationError> {
        panic!("no absent alert claim may be delivered")
    }
}

pub fn consumers() -> (
    DeadlineRuntimeConfig,
    AlertConsumers,
    crate::serve_report_composition::ReportConsumer,
) {
    let alerts = Arc::new(Alerts);
    (
        DeadlineRuntimeConfig::new(
            DeadlineDispatchLimit::new(1).unwrap(),
            Duration::from_millis(1),
        )
        .unwrap(),
        AlertConsumers {
            scheduler: alerts.clone(),
            delivery: alerts,
            sender: None,
            config: AlertRuntimeConfig::new(1, Duration::from_millis(1)).unwrap(),
        },
        Arc::new(|| Ok(application::case_reports::CaseReportWorkerRun::Idle)),
    )
}

pub fn server(
    drops: &Drops,
    stop: Arc<Stop>,
    budget: HttpWorkBudget,
    calls: Arc<AtomicUsize>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
) -> (
    ServerComponents,
    crate::serve_password_reset_runtime::PasswordResetAdmission,
) {
    let probe = Probe::new("reset-owner", drops);
    let release = Mutex::new(release);
    let owner: ResetRequestWork = Arc::new(move |email: Zeroizing<String>| {
        let _keep_owner = &probe;
        assert!(email.as_str() == "fixture@example.test");
        calls.fetch_add(1, SeqCst);
        entered.send(()).expect("request observer present");
        release
            .lock()
            .unwrap()
            .recv_timeout(WATCHDOG * 2)
            .expect("request release watchdog");
        Ok(application::identity::password_reset::ResetRequestAccepted)
    });
    let (admission, consumer) = PasswordResetConsumer::new(owner.clone(), budget, stop.clone());
    assert!(matches!(
        admission.try_submit(Zeroizing::new("fixture@example.test".into())),
        RequestAdmission::Accepted
    ));
    let router = Router::new()
        .route("/drop-probe", get(|| async { "probe" }))
        .layer(Extension(Arc::new(Probe::new("router-owner", drops))));
    let server = ServerComponents {
        router,
        stop,
        password_reset: Some(PasswordResetRuntime { owner, consumer }),
    };
    (server, admission)
}

pub fn assert_drops(drops: &Drops, owner: ThreadId) {
    let mut drops = drops.lock().unwrap();
    drops.sort_by_key(|value| value.label);
    assert_eq!(
        drops.iter().map(|value| value.label).collect::<Vec<_>>(),
        ["reset-owner", "router-owner"]
    );
    for value in drops.iter() {
        assert!(!value.in_runtime, "{} dropped inside Tokio", value.label);
        assert_eq!(
            value.thread, owner,
            "{} escaped its synchronous owner",
            value.label
        );
    }
}
