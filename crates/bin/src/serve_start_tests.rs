//! Preserve synchronous adapter ownership through the complete server lifetime.

use crate::{
    serve_alert_composition::AlertConsumers, serve_alert_runtime::AlertRuntimeConfig,
    serve_deadline_runtime::DeadlineRuntimeConfig, serve_start,
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

#[derive(Debug)]
struct Dropped {
    label: &'static str,
    inside_runtime: bool,
    thread: ThreadId,
}

type Drops = Arc<Mutex<Vec<Dropped>>>;

struct Probe {
    label: &'static str,
    drops: Drops,
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.lock().unwrap().push(Dropped {
            label: self.label,
            inside_runtime: tokio::runtime::Handle::try_current().is_ok(),
            thread: thread::current().id(),
        });
    }
}

struct Dispatcher {
    _probe: Probe,
    calls: Arc<AtomicUsize>,
}

impl DeadlineDispatchStore for Dispatcher {
    fn dispatch(
        &self,
        _: DeadlineDispatchRequest,
    ) -> Result<DeadlineDispatchBatch, ApplicationError> {
        self.calls.fetch_add(1, SeqCst);
        Err(ApplicationError::InvalidConfiguration(
            "scripted consumer stop".into(),
        ))
    }
}

struct Worker {
    _probe: Probe,
    calls: Arc<AtomicUsize>,
}

impl DeadlineWorkerStore for Worker {
    fn run_next(&self) -> Result<DeadlineWorkerRun, ApplicationError> {
        self.calls.fetch_add(1, SeqCst);
        Ok(DeadlineWorkerRun::Idle)
    }

    fn result(&self, _: Uuid) -> Result<Option<DeadlineWorkerResult>, ApplicationError> {
        panic!("composition must not read individual worker results")
    }

    fn latest_attempt(&self, _: Uuid) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
        panic!("composition must not read individual worker attempts")
    }
}

struct Alerts {
    _probe: Probe,
    calls: Arc<AtomicUsize>,
}

impl AlertSchedulerStore for Alerts {
    fn run_next(&self) -> Result<AlertSchedulerRun, ApplicationError> {
        self.calls.fetch_add(1, SeqCst);
        Ok(AlertSchedulerRun::Idle)
    }
}

impl AlertDeliveryStore for Alerts {
    fn claim_next(&self) -> Result<Option<AlertDeliveryClaim>, ApplicationError> {
        self.calls.fetch_add(1, SeqCst);
        Ok(None)
    }

    fn complete_attempt(&self, _: AlertDeliveryCompletion) -> Result<(), ApplicationError> {
        panic!("composition must not complete an absent claim")
    }
}

struct Sender {
    _probe: Probe,
}

impl AlertEmailSender for Sender {
    fn send(&self, _: &AlertEmailMessage) -> AlertEmailOutcome {
        panic!("composition must not send an absent claim")
    }
}

#[test]
fn failed_bind_starts_no_consumer_and_drops_every_final_owner_outside_tokio() {
    scenario("127.0.0.1:not-a-port", false);
}

#[test]
fn consumer_failure_drains_router_and_drops_final_adapters_on_the_sync_owner_thread() {
    scenario("127.0.0.1:0", true);
}

fn scenario(bind: &'static str, consumer_started: bool) {
    let drops: Drops = Arc::new(Mutex::new(Vec::new()));
    let dispatch_calls = Arc::new(AtomicUsize::new(0));
    let worker_calls = Arc::new(AtomicUsize::new(0));
    let alert_calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&drops);
    let dispatched = Arc::clone(&dispatch_calls);
    let worked = Arc::clone(&worker_calls);
    let alerted = Arc::clone(&alert_calls);
    let (sender, finished) = mpsc::sync_channel(1);
    let owner = thread::spawn(move || {
        let probe = |label| Probe {
            label,
            drops: Arc::clone(&observed),
        };
        // No independent Arc<Probe> remains to hide the router's final drop.
        let router = Router::new()
            .route("/drop-probe", get(|| async { "probe" }))
            .layer(Extension(Arc::new(probe("router"))));
        let dispatch = Dispatcher {
            _probe: probe("dispatcher"),
            calls: dispatched,
        };
        let worker = Worker {
            _probe: probe("worker"),
            calls: worked,
        };
        let config = DeadlineRuntimeConfig::new(
            DeadlineDispatchLimit::new(1).unwrap(),
            Duration::from_millis(1),
        )
        .unwrap();
        let alerts = Arc::new(Alerts {
            _probe: probe("alerts"),
            calls: alerted,
        });
        let consumers = AlertConsumers {
            scheduler: alerts.clone(),
            delivery: alerts,
            sender: Some(Arc::new(Sender {
                _probe: probe("sender"),
            })),
            config: AlertRuntimeConfig::new(1, Duration::from_millis(1)).unwrap(),
        };
        let result = serve_start::run(bind, router, dispatch, worker, config, consumers);
        let _ = sender.send((thread::current().id(), result));
    });
    let (owner_thread, result) = finished
        .recv_timeout(Duration::from_secs(5))
        .expect("composition must finish after bind or fatal consumer failure");
    owner.join().unwrap();
    let error = result.expect_err("the scripted exit must remain an error");
    if consumer_started {
        assert!(error.to_string().contains("consumer_failed"));
    }
    assert_eq!(dispatch_calls.load(SeqCst), usize::from(consumer_started));
    assert_eq!(worker_calls.load(SeqCst), 0);
    if !consumer_started {
        assert_eq!(alert_calls.load(SeqCst), 0);
    }
    let mut events = drops.lock().unwrap();
    events.sort_by_key(|event| event.label);
    assert_eq!(
        events.iter().map(|event| event.label).collect::<Vec<_>>(),
        vec!["alerts", "dispatcher", "router", "sender", "worker"]
    );
    for event in events.iter() {
        assert!(
            !event.inside_runtime,
            "final {} owner dropped inside Tokio",
            event.label
        );
        assert_eq!(
            event.thread, owner_thread,
            "final {} owner escaped its sync scope",
            event.label
        );
    }
}
