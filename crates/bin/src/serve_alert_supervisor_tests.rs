use super::supervise;
use crate::{serve_deadline_runtime::RuntimeControl, serve_stop::Stop};
use application::{alerts::AlertError, ApplicationError};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::{sync::oneshot, task::AbortHandle};

const WATCHDOG: Duration = Duration::from_secs(3);
async fn finished(handle: &AbortHandle) {
    tokio::time::timeout(WATCHDOG, async {
        while !handle.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("consumer completion watchdog");
}

#[tokio::test]
async fn every_unexpected_consumer_exit_requests_stop_and_joins_the_other_before_returning() {
    for deadlines_first in [true, false] {
        for exit in 0..3 {
            let stop = Arc::new(Stop::default());
            let first = tokio::spawn(async move {
                match exit {
                    0 => Ok(()),
                    1 => Err(ApplicationError::from(AlertError::Stored(
                        "private-consumer-failure".into(),
                    ))),
                    _ => panic!("private-consumer-panic"),
                }
            });
            let first_handle = first.abort_handle();
            let (release, hold) = oneshot::channel();
            let done = Arc::new(AtomicBool::new(false));
            let done_copy = done.clone();
            let second = tokio::spawn(async move {
                let _ = hold.await;
                done_copy.store(true, SeqCst);
                Ok(())
            });
            let second_handle = second.abort_handle();
            finished(&first_handle).await;
            let (deadline, alert) = if deadlines_first {
                (first, second)
            } else {
                (second, first)
            };
            let mut future = Box::pin(supervise(deadline, alert, stop.clone()));
            let initial = future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()));
            let pending = initial.is_pending();
            let stopped = stop.is_stopped();
            let joined_early = done.load(SeqCst);
            let _ = release.send(());
            let result = match initial {
                Poll::Ready(result) => result,
                Poll::Pending => tokio::time::timeout(WATCHDOG, future)
                    .await
                    .expect("supervisor watchdog"),
            };
            finished(&second_handle).await;
            assert!(pending, "supervisor abandoned the other consumer");
            assert!(stopped);
            assert!(!joined_early);
            assert!(done.load(SeqCst));
            assert!(result.is_err(), "unexpected exit became success");
        }
    }
}

#[tokio::test]
async fn requested_stop_waits_for_both_owners_and_reports_normal_completion() {
    let stop = Arc::new(Stop::default());
    let (release_deadline, deadline_hold) = oneshot::channel();
    let (release_alert, alert_hold) = oneshot::channel();
    let deadlines = tokio::spawn(async move {
        let _ = deadline_hold.await;
        Ok(())
    });
    let alerts = tokio::spawn(async move {
        let _ = alert_hold.await;
        Ok(())
    });
    let deadline_handle = deadlines.abort_handle();
    let alert_handle = alerts.abort_handle();
    stop.request();
    let mut future = Box::pin(supervise(deadlines, alerts, stop));
    let initial = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let initially_pending = initial.is_pending();
    let _ = release_deadline.send(());
    finished(&deadline_handle).await;
    let after_first = if initially_pending {
        Some(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
        )
    } else {
        None
    };
    let kept_alert = after_first.as_ref().is_some_and(Poll::is_pending);
    let _ = release_alert.send(());
    let result = match initial {
        Poll::Ready(result) => result,
        Poll::Pending => match after_first.expect("polled active supervisor") {
            Poll::Ready(result) => result,
            Poll::Pending => tokio::time::timeout(WATCHDOG, future)
                .await
                .expect("supervisor watchdog"),
        },
    };
    finished(&alert_handle).await;
    assert!(initially_pending);
    assert!(kept_alert, "deadline completion detached alert owner");
    result.unwrap();
}
