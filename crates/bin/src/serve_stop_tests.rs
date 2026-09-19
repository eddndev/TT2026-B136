use super::*;
use std::{
    sync::{mpsc, Arc, Barrier},
    thread,
    time::Instant,
};

const WATCHDOG: Duration = Duration::from_secs(3);
const LONG_PAUSE: Duration = Duration::from_secs(3600);

#[test]
fn stop_request_is_shared_sticky_and_idempotent() {
    let stop = Arc::new(Stop::default());
    let other = Arc::clone(&stop);
    assert!(!stop.is_stopped());
    other.request();
    assert!(stop.is_stopped());
    stop.request();
    assert!(other.is_stopped());
}

#[test]
fn request_before_wait_cannot_lose_the_wakeup() {
    let stop = Arc::new(Stop::default());
    stop.request();
    let worker_stop = Arc::clone(&stop);
    let (done_tx, done) = mpsc::sync_channel(1);
    let waiter = thread::spawn(move || {
        worker_stop.wait(LONG_PAUSE);
        done_tx.send(worker_stop.is_stopped()).unwrap();
    });
    let result = done.recv_timeout(WATCHDOG);
    stop.request();
    assert!(result.expect("pre-stopped wait watchdog"));
    waiter.join().unwrap();
}

#[test]
fn request_releases_all_concurrent_waiters_and_future_waits() {
    let stop = Arc::new(Stop::default());
    let start = Arc::new(Barrier::new(4));
    let (done_tx, done) = mpsc::channel();
    let mut waiters = Vec::new();
    for index in 0..3 {
        let (stop, start, done_tx) = (Arc::clone(&stop), Arc::clone(&start), done_tx.clone());
        waiters.push(thread::spawn(move || {
            start.wait();
            stop.wait(LONG_PAUSE);
            let first = stop.is_stopped();
            stop.wait(LONG_PAUSE);
            done_tx.send((index, first, stop.is_stopped())).unwrap();
        }));
    }
    drop(done_tx);
    start.wait();
    stop.request();
    let mut results = Vec::new();
    for _ in 0..3 {
        results.push(done.recv_timeout(WATCHDOG).expect("all waiters must wake"));
    }
    for waiter in waiters {
        waiter.join().unwrap();
    }
    results.sort_by_key(|value| value.0);
    assert_eq!(
        results,
        vec![(0, true, true), (1, true, true), (2, true, true)]
    );
}

#[test]
fn notifications_without_stop_do_not_shorten_the_minimum_pause() {
    let stop = Arc::new(Stop::default());
    let worker_stop = Arc::clone(&stop);
    let start = Arc::new(Barrier::new(2));
    let worker_start = Arc::clone(&start);
    let poll = Duration::from_millis(20);
    let (done_tx, done) = mpsc::sync_channel(1);
    let waiter = thread::spawn(move || {
        worker_start.wait();
        let began = Instant::now();
        worker_stop.wait(poll);
        done_tx
            .send((began.elapsed(), worker_stop.is_stopped()))
            .unwrap();
    });
    start.wait();
    let watchdog = Instant::now();
    let mut notifications = 0;
    let outcome = loop {
        // A notification alone must not change the predicate or the deadline.
        stop.wake.notify_all();
        notifications += 1;
        match done.try_recv() {
            Ok(value) => break Some(value),
            Err(mpsc::TryRecvError::Disconnected) => break None,
            Err(mpsc::TryRecvError::Empty) if watchdog.elapsed() < WATCHDOG => thread::yield_now(),
            Err(mpsc::TryRecvError::Empty) => break None,
        }
    };
    stop.request();
    outcome
        .or_else(|| done.recv_timeout(WATCHDOG).ok())
        .expect("waiter stop watchdog");
    waiter.join().unwrap();
    let (elapsed, stopped) = outcome.expect("minimum pause watchdog");
    assert!(notifications > 0);
    assert!(!stopped, "ordinary notifications must not request stop");
    assert!(
        elapsed >= poll,
        "notification shortened {poll:?} to {elapsed:?}"
    );
}
