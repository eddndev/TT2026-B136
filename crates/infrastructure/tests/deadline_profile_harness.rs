#[path = "deadline_profile_interleaved_support/rendezvous.rs"]
mod rendezvous;

use rendezvous::PrepareRendezvous;
use std::{
    panic::catch_unwind,
    process::{Command, Stdio},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

const CHILD_MODE: &str = "TT_PROFILE_PREPARE_FAILURE";

#[test]
fn early_worker_error_does_not_leave_its_peer_blocked() {
    assert_child_completes("error");
}

#[test]
fn early_worker_panic_does_not_leave_its_peer_blocked() {
    assert_child_completes("panic");
}

#[test]
fn both_prepared_workers_are_released_together() {
    assert_child_completes("pair");
}

#[test]
fn a_peer_arriving_after_timeout_is_rejected() {
    assert_child_completes("late");
}

fn assert_child_completes(mode: &str) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "prepare_failure_child", "--nocapture"])
        .env(CHILD_MODE, mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let completed = loop {
        if child.try_wait().unwrap().is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = child.wait_with_output().unwrap();
    assert!(
        completed,
        "peer remained blocked after an early worker {mode}; child was killed and reaped"
    );
    assert!(
        output.status.success(),
        "child rejected {mode}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn prepare_failure_child() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        return;
    };
    match mode.as_str() {
        "error" | "panic" => failed_prepare(mode),
        "pair" => successful_pair(),
        "late" => late_peer(),
        _ => panic!("unsupported child mode"),
    }
}

fn failed_prepare(mode: String) {
    let gate = Arc::new(PrepareRendezvous::new(Duration::from_millis(100)));
    let (entered_tx, entered_rx) = mpsc::channel();
    let peer = thread::spawn(move || {
        entered_tx.send(()).unwrap();
        catch_unwind(|| gate.wait())
    });
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    // An unsuccessful prepare never reaches the shared post-prepare hook.
    let failure = thread::spawn(move || -> Result<(), &'static str> {
        match mode.as_str() {
            "error" => Err("injected prepare error"),
            "panic" => panic!("injected prepare panic"),
            _ => panic!("unsupported child mode"),
        }
    });
    assert!(!matches!(failure.join(), Ok(Ok(()))));
    assert!(
        peer.join().unwrap().is_err(),
        "a missing peer must fail the rendezvous instead of allowing a commit"
    );
}

fn successful_pair() {
    let gate = Arc::new(PrepareRendezvous::new(Duration::from_secs(2)));
    let copy = gate.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let peer = thread::spawn(move || {
        entered_tx.send(()).unwrap();
        finished_tx
            .send(catch_unwind(|| copy.wait()).is_ok())
            .unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let premature = finished_rx.recv_timeout(Duration::from_millis(30));
    let second = catch_unwind(|| gate.wait());
    let first = finished_rx.recv_timeout(Duration::from_secs(3));
    peer.join().unwrap();
    assert_eq!(premature, Err(mpsc::RecvTimeoutError::Timeout));
    assert!(second.is_ok());
    assert_eq!(first, Ok(true));
}

fn late_peer() {
    let gate = Arc::new(PrepareRendezvous::new(Duration::from_millis(100)));
    let copy = gate.clone();
    let peer = thread::spawn(move || catch_unwind(|| copy.wait()));
    let first = peer.join().unwrap();
    let late = catch_unwind(|| gate.wait());
    assert!(first.is_err());
    assert!(
        late.is_err(),
        "a failed rendezvous must reject a late participant"
    );
}
