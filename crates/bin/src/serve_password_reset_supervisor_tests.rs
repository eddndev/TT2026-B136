use crate::{
    serve_alert_supervisor,
    serve_deadline_runtime::RuntimeControl,
    serve_password_reset_runtime_test_support::{poll, WATCHDOG},
    serve_stop::Stop,
};
use application::ApplicationError;
use std::sync::Arc;
use tokio::{sync::oneshot, task::JoinHandle};

struct Held {
    release: oneshot::Sender<()>,
    task: JoinHandle<Result<(), ApplicationError>>,
}

fn held() -> Held {
    let (release, waiting) = oneshot::channel();
    let task = tokio::spawn(async move {
        waiting
            .await
            .expect("supervisor test releases every consumer");
        Ok(())
    });
    Held { release, task }
}

#[tokio::test]
async fn disabled_reset_adds_no_immediate_exit_to_existing_consumer_supervision() {
    let deadlines = held();
    let alerts = held();
    let reports = held();
    let stop = Arc::new(Stop::default());
    let mut supervisor = Box::pin(serve_alert_supervisor::supervise_with_password_reset(
        deadlines.task,
        alerts.task,
        reports.task,
        None,
        Arc::clone(&stop),
    ));
    assert!(poll(supervisor.as_mut()).is_pending());
    assert!(
        !stop.is_stopped(),
        "disabled reset caused an unexpected consumer exit"
    );
    stop.request();
    deadlines.release.send(()).unwrap();
    alerts.release.send(()).unwrap();
    reports.release.send(()).unwrap();
    tokio::time::timeout(WATCHDOG, supervisor)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn reset_supervision_joins_all_existing_consumers_after_its_fatal_exit() {
    let deadlines = held();
    let alerts = held();
    let reports = held();
    let stop = Arc::new(Stop::default());
    let (reset_release, released) = oneshot::channel();
    let reset = tokio::spawn(async move {
        released.await.unwrap();
        Err(ApplicationError::Port(
            "private-reset-consumer-message".into(),
        ))
    });
    let mut supervisor = Box::pin(serve_alert_supervisor::supervise_with_password_reset(
        deadlines.task,
        alerts.task,
        reports.task,
        Some(reset),
        Arc::clone(&stop),
    ));
    assert!(poll(supervisor.as_mut()).is_pending());
    reset_release.send(()).unwrap();
    let stopped = async {
        tokio::select! {
            result = &mut supervisor => {
                assert!(result.is_err());
                panic!("fatal reset consumer detached held siblings");
            }
            () = stop.stopped() => {}
        }
    };
    tokio::time::timeout(WATCHDOG, stopped)
        .await
        .expect("stop notification watchdog");
    assert!(stop.is_stopped());
    assert!(poll(supervisor.as_mut()).is_pending());
    deadlines.release.send(()).unwrap();
    alerts.release.send(()).unwrap();
    assert!(
        poll(supervisor.as_mut()).is_pending(),
        "report consumer join was abandoned"
    );
    reports.release.send(()).unwrap();
    let error = tokio::time::timeout(WATCHDOG, supervisor)
        .await
        .unwrap()
        .expect_err("reset failure must remain visible after drainage");
    assert!(error.to_string().contains("password_reset_failed"));
    assert!(!error.to_string().contains("private-reset-consumer-message"));
}
