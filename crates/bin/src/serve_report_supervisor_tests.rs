//! Report consumers share stop, drainage and final-owner guarantees.
use super::supervise;
use crate::{serve_deadline_runtime::RuntimeControl, serve_runtime, serve_stop::Stop};
use application::ApplicationError;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::{
    sync::oneshot,
    task::{AbortHandle, JoinHandle},
};

const WATCHDOG: Duration = Duration::from_secs(3);
type Consumer = JoinHandle<Result<(), ApplicationError>>;
async fn finished(handle: &AbortHandle) {
    tokio::time::timeout(WATCHDOG, async {
        while !handle.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("consumer completion watchdog");
}
fn held() -> (oneshot::Sender<()>, Consumer, Arc<AtomicBool>) {
    let (release, hold) = oneshot::channel();
    let done = Arc::new(AtomicBool::new(false));
    let observed = done.clone();
    let consumer = tokio::spawn(async move {
        let _ = hold.await;
        observed.store(true, SeqCst);
        Ok(())
    });
    (release, consumer, done)
}

#[tokio::test]
async fn report_error_panic_or_unexpected_success_stops_and_joins_both_other_consumers() {
    for (exit, code) in [
        (0, "reports_stopped"),
        (1, "reports_failed"),
        (2, "reports_panicked"),
    ] {
        let stop = Arc::new(Stop::default());
        let reports = tokio::spawn(async move {
            match exit {
                0 => Ok(()),
                1 => Err(ApplicationError::InvalidConfiguration(
                    "private-report-secret".into(),
                )),
                _ => panic!("private-report-secret"),
            }
        });
        finished(&reports.abort_handle()).await;
        let (release_deadline, deadlines, deadline_done) = held();
        let (release_alert, alerts, alert_done) = held();
        let deadline_handle = deadlines.abort_handle();
        let alert_handle = alerts.abort_handle();
        let mut future = Box::pin(supervise(deadlines, alerts, reports, stop.clone()));
        let initial = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        let pending = initial.is_pending();
        let stopped = stop.is_stopped();
        let joined_early = deadline_done.load(SeqCst) || alert_done.load(SeqCst);
        let _ = release_deadline.send(());
        finished(&deadline_handle).await;
        let after_deadline = if pending {
            Some(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
            )
        } else {
            None
        };
        let held_alert = after_deadline.as_ref().is_some_and(Poll::is_pending);
        let _ = release_alert.send(());
        let result = match initial {
            Poll::Ready(result) => result,
            Poll::Pending => match after_deadline.expect("polled active supervisor") {
                Poll::Ready(result) => result,
                Poll::Pending => tokio::time::timeout(WATCHDOG, future)
                    .await
                    .expect("supervisor watchdog"),
            },
        };
        finished(&alert_handle).await;
        assert!(pending && stopped && !joined_early);
        assert!(held_alert, "report exit detached in-flight alert work");
        assert!(deadline_done.load(SeqCst) && alert_done.load(SeqCst));
        let error = result.unwrap_err().to_string();
        assert!(error.contains(code), "wrong consumer result: {error}");
        assert!(!error.contains("private-report-secret"));
    }
}

#[tokio::test]
async fn requested_stop_retains_the_report_join_after_other_consumers_finish() {
    let stop = Arc::new(Stop::default());
    stop.request();
    let deadlines = tokio::spawn(async { Ok(()) });
    let alerts = tokio::spawn(async { Ok(()) });
    finished(&deadlines.abort_handle()).await;
    finished(&alerts.abort_handle()).await;
    let (release_report, reports, report_done) = held();
    let report_handle = reports.abort_handle();
    let mut future = Box::pin(supervise(deadlines, alerts, reports, stop));
    let initial = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let pending = initial.is_pending();
    let detached_early = report_done.load(SeqCst);
    let _ = release_report.send(());
    let result = match initial {
        Poll::Ready(value) => value,
        Poll::Pending => tokio::time::timeout(WATCHDOG, future)
            .await
            .expect("report drainage watchdog"),
    };
    finished(&report_handle).await;
    assert!(
        pending && !detached_early,
        "requested stop abandoned the report owner"
    );
    assert!(report_done.load(SeqCst));
    result.unwrap();
}

#[tokio::test]
async fn report_failure_drains_other_consumers_and_http_through_the_existing_server_supervisor() {
    let stop = Arc::new(Stop::default());
    let reports = tokio::spawn(async {
        Err(ApplicationError::InvalidConfiguration(
            "private-report-error".into(),
        ))
    });
    finished(&reports.abort_handle()).await;
    let (release_deadline, deadlines, _) = held();
    let (release_alert, alerts, _) = held();
    let mut consumer_future = Box::pin(supervise(deadlines, alerts, reports, stop.clone()));
    assert!(consumer_future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending());
    let stop_before_drain = stop.is_stopped();
    let consumer = tokio::spawn(consumer_future);
    let consumer_handle = consumer.abort_handle();
    let (http_shutdown, shutdown) = oneshot::channel();
    let (release_http, hold_http) = oneshot::channel();
    let shutdown_seen = Arc::new(AtomicBool::new(false));
    let http_done = Arc::new(AtomicBool::new(false));
    let notified = shutdown_seen.clone();
    let drained = http_done.clone();
    let http = async move {
        let _ = shutdown.await;
        notified.store(true, SeqCst);
        let _ = hold_http.await;
        drained.store(true, SeqCst);
        Ok(())
    };
    let signal = std::future::pending::<std::io::Result<serve_runtime::ShutdownSignal>>();
    let mut future = Box::pin(serve_runtime::supervise(
        http,
        consumer,
        stop,
        http_shutdown,
        signal,
    ));
    assert!(future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending());
    let _ = release_deadline.send(());
    let _ = release_alert.send(());
    finished(&consumer_handle).await;
    let after_consumer = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let waits_http = after_consumer.is_pending();
    let saw_shutdown = shutdown_seen.load(SeqCst);
    let drained_early = http_done.load(SeqCst);
    let _ = release_http.send(());
    let result = match after_consumer {
        Poll::Ready(value) => value,
        Poll::Pending => tokio::time::timeout(WATCHDOG, future)
            .await
            .expect("HTTP drainage watchdog"),
    };
    assert!(
        stop_before_drain,
        "report failure did not stop sibling consumers"
    );
    assert!(waits_http && saw_shutdown && !drained_early);
    assert!(http_done.load(SeqCst));
    let error = result.unwrap_err().to_string();
    assert!(error.contains("consumer_failed"));
    assert!(!error.contains("private-report-error"));
}
