use crate::{
    serve_deadline_runtime::RuntimeControl, serve_password_reset_start_support::*, serve_start,
    serve_stop::Stop,
};
use std::{
    future::Future,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        mpsc, Arc, Mutex,
    },
    task::{Context, Waker},
    thread,
};
use web::{
    password_reset::{PasswordResetRequests, RequestAdmission},
    HttpWorkBudget,
};
use zeroize::Zeroizing;

#[test]
fn failed_bind_runs_no_queued_recovery_and_drops_its_final_owner_outside_tokio() {
    let drops: Drops = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(AtomicUsize::new(0));
    let observed_drops = drops.clone();
    let observed_calls = calls.clone();
    let (finished_tx, finished) = mpsc::channel();
    let owner = thread::spawn(move || {
        let (entered, _entry) = mpsc::channel();
        let (_release, released) = mpsc::channel();
        let (_fail, failed) = mpsc::channel();
        let (server, admission) = server(
            &observed_drops,
            Arc::new(Stop::default()),
            HttpWorkBudget::new(NonZeroUsize::new(1).unwrap()),
            observed_calls,
            entered,
            released,
        );
        let (config, alerts, reports) = consumers();
        let result = serve_start::run(
            "127.0.0.1:not-a-port",
            server,
            Dispatcher {
                fail: Mutex::new(failed),
            },
            Worker,
            config,
            alerts,
            reports,
        );
        assert!(matches!(
            admission.try_submit(Zeroizing::new("fixture@example.test".into())),
            RequestAdmission::Unavailable
        ));
        finished_tx.send((thread::current().id(), result)).unwrap();
    });
    let (owner_id, result) = finished
        .recv_timeout(WATCHDOG)
        .expect("failed bind watchdog");
    owner.join().unwrap();
    assert!(result
        .expect_err("invalid bind must fail")
        .to_string()
        .contains("cannot bind"));
    assert_eq!(calls.load(SeqCst), 0);
    assert_drops(&drops, owner_id);
}

#[test]
fn real_start_shares_stop_and_retains_recovery_budget_and_owners_until_drained() {
    let drops: Drops = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(Stop::default());
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let (entered, entry) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (fail_dispatch, failed) = mpsc::channel();
    let (admission_tx, admission_rx) = mpsc::channel();
    let (finished_tx, finished) = mpsc::channel();
    let owned_drops = drops.clone();
    let owned_calls = calls.clone();
    let owned_stop = stop.clone();
    let owned_budget = budget.clone();
    let owner = thread::spawn(move || {
        let (server, admission) = server(
            &owned_drops,
            owned_stop,
            owned_budget,
            owned_calls,
            entered,
            released,
        );
        admission_tx.send(admission).unwrap();
        let (config, alerts, reports) = consumers();
        let result = serve_start::run(
            "127.0.0.1:0",
            server,
            Dispatcher {
                fail: Mutex::new(failed),
            },
            Worker,
            config,
            alerts,
            reports,
        );
        finished_tx.send((thread::current().id(), result)).unwrap();
    });
    let admission = admission_rx.recv_timeout(WATCHDOG).unwrap();
    entry
        .recv_timeout(WATCHDOG)
        .expect("recovery must start after binding");
    fail_dispatch.send(()).unwrap();
    stop.wait(WATCHDOG);
    let stopped = stop.is_stopped();
    let admission_closed = matches!(
        admission.try_submit(Zeroizing::new("fixture@example.test".into())),
        RequestAdmission::Unavailable
    );
    let mut next_http = Box::pin(budget.acquire_owned());
    let budget_held = next_http
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending();
    let early_result = finished.try_recv().ok();
    let joined_early = early_result.is_some();
    let dropped_early = !drops.lock().unwrap().is_empty();
    release.send(()).unwrap();
    let (owner_id, result) = early_result
        .or_else(|| finished.recv_timeout(WATCHDOG).ok())
        .expect("drain watchdog");
    owner.join().unwrap();
    assert!(stopped, "server created an independent stop state");
    assert!(admission_closed);
    assert!(
        budget_held,
        "in-flight recovery released the shared HTTP permit"
    );
    assert!(
        !joined_early,
        "server detached its in-flight recovery request"
    );
    assert!(
        !dropped_early,
        "an adapter owner was released before drainage"
    );
    assert_eq!(calls.load(SeqCst), 1);
    assert!(result
        .expect_err("dispatcher failed")
        .to_string()
        .contains("consumer_failed"));
    assert_drops(&drops, owner_id);
}
