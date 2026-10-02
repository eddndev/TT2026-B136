use crate::{
    serve_deadline_runtime::RuntimeControl,
    serve_password_reset_runtime_test_support::{poll, Outcome, Running, PRIVATE_ERROR, WATCHDOG},
};
use std::sync::atomic::Ordering::SeqCst;
use web::password_reset::RequestAdmission;

#[test]
fn simultaneous_admission_clones_accept_at_most_one_unstarted_request() {
    use std::sync::{Arc, Barrier};
    use web::password_reset::PasswordResetRequests;
    let running = Running::new(1, Outcome::Accepted);
    let gate = Arc::new(Barrier::new(3));
    let tasks: Vec<_> = (0..2)
        .map(|_| {
            let admission = running.admission.clone();
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                gate.wait();
                admission.try_submit(zeroize::Zeroizing::new("fixture@example.test".into()))
            })
        })
        .collect();
    gate.wait();
    let results: Vec<_> = tasks.into_iter().map(|task| task.join().unwrap()).collect();
    assert_eq!(
        results
            .iter()
            .filter(|value| matches!(value, RequestAdmission::Accepted))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|value| matches!(value, RequestAdmission::Busy))
            .count(),
        1
    );
    assert_eq!(running.work.count(), 0);
}

#[tokio::test]
async fn idle_consumer_reserves_no_shared_slot_and_stops_without_work() {
    let mut running = Running::new(1, Outcome::Accepted);
    assert_eq!(running.budget.capacity(), 1);
    assert!(running.pending());
    let mut permit = Box::pin(running.budget.acquire_owned());
    let slot = match poll(permit.as_mut()) {
        std::task::Poll::Ready(value) => value.expect("live shared budget"),
        std::task::Poll::Pending => panic!("idle reset consumer reserved HTTP capacity"),
    };
    assert_eq!(running.work.count(), 0);
    running.stop.request();
    assert!(matches!(running.submit(), RequestAdmission::Unavailable));
    drop(slot);
    drop(permit);
    running.finish().await.unwrap();
}

#[tokio::test]
async fn one_request_total_waits_for_the_same_http_budget_then_owns_its_slot() {
    let mut running = Running::new(2, Outcome::Accepted);
    let budget = running.budget.clone();
    let first_http = budget.acquire_owned().await.unwrap();
    let second_http = budget.acquire_owned().await.unwrap();
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    assert!(matches!(running.submit(), RequestAdmission::Busy));
    assert!(running.pending());
    assert_eq!(running.work.count(), 0);
    drop(second_http);
    running.entered().await;
    assert_eq!(running.work.count(), 1);
    assert!(matches!(running.submit(), RequestAdmission::Busy));
    let mut next_http = Box::pin(budget.acquire_owned());
    assert!(poll(next_http.as_mut()).is_pending());
    running.stop.request();
    assert!(running.pending(), "stop detached synchronous work");
    assert!(
        poll(next_http.as_mut()).is_pending(),
        "stop freed an active slot"
    );
    assert!(matches!(running.submit(), RequestAdmission::Unavailable));
    running.work.release();
    running.finish().await.unwrap();
    drop(
        tokio::time::timeout(WATCHDOG, next_http)
            .await
            .unwrap()
            .unwrap(),
    );
    drop(first_http);
}

#[tokio::test]
async fn stop_discards_a_queued_request_without_starting_account_work() {
    let running = Running::new(1, Outcome::Accepted);
    let calls = running.work.calls.clone();
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    running.stop.request();
    assert!(matches!(running.submit(), RequestAdmission::Unavailable));
    running.finish().await.unwrap();
    assert_eq!(calls.load(SeqCst), 0);
}

#[tokio::test]
async fn stop_wins_even_when_shared_capacity_becomes_available_before_next_poll() {
    let mut running = Running::new(1, Outcome::Accepted);
    let budget = running.budget.clone();
    let held = budget.acquire_owned().await.unwrap();
    let calls = running.work.calls.clone();
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    assert!(running.pending());
    assert_eq!(calls.load(SeqCst), 0);
    running.stop.request();
    drop(held);
    running.finish().await.unwrap();
    assert_eq!(calls.load(SeqCst), 0, "request began after stop had won");
    let slot = tokio::time::timeout(WATCHDOG, budget.acquire_owned())
        .await
        .unwrap()
        .unwrap();
    drop(slot);
}

#[tokio::test]
async fn dropping_an_unstarted_consumer_closes_all_admission_handles() {
    let running = Running::new(1, Outcome::Accepted);
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    let admission = running.admission.clone();
    let calls = running.work.calls.clone();
    drop(running);
    use web::password_reset::PasswordResetRequests;
    assert!(matches!(
        admission.try_submit(zeroize::Zeroizing::new("fixture@example.test".into())),
        RequestAdmission::Unavailable
    ));
    assert_eq!(calls.load(SeqCst), 0);
}

#[tokio::test]
async fn draining_a_request_error_preserves_ownership_without_retry_or_leaking_details() {
    let mut running = Running::new(1, Outcome::Error);
    let calls = running.work.calls.clone();
    let budget = running.budget.clone();
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    running.entered().await;
    running.stop.request();
    assert!(running.pending());
    let mut blocked = Box::pin(budget.acquire_owned());
    assert!(poll(blocked.as_mut()).is_pending());
    running.work.release();
    running
        .finish()
        .await
        .expect("request error does not turn graceful stop into fatal exit");
    assert_eq!(calls.load(SeqCst), 1);
    drop(
        tokio::time::timeout(WATCHDOG, blocked)
            .await
            .unwrap()
            .unwrap(),
    );
}

#[tokio::test]
async fn work_panic_stops_admission_and_returns_only_a_neutral_supervision_error() {
    let mut running = Running::new(1, Outcome::Panic);
    let admission = running.admission.clone();
    let stop = running.stop.clone();
    let calls = running.work.calls.clone();
    let budget = running.budget.clone();
    assert!(matches!(running.submit(), RequestAdmission::Accepted));
    running.entered().await;
    running.work.release();
    let error = running
        .finish()
        .await
        .expect_err("panicked worker must be fatal");
    assert!(!error.to_string().contains(PRIVATE_ERROR));
    assert!(!error.to_string().contains("fixture@example.test"));
    assert!(stop.is_stopped());
    assert_eq!(calls.load(SeqCst), 1);
    use web::password_reset::PasswordResetRequests;
    assert!(matches!(
        admission.try_submit(zeroize::Zeroizing::new("fixture@example.test".into())),
        RequestAdmission::Unavailable
    ));
    drop(
        tokio::time::timeout(WATCHDOG, budget.acquire_owned())
            .await
            .unwrap()
            .unwrap(),
    );
}
