use super::support::*;
use application::{deadline_dispatch::DeadlineDispatchStream::*, ApplicationError};
use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
};

fn spawn(
    harness: Harness,
    limit: u32,
) -> (JoinHandle<()>, mpsc::Receiver<Result<(), ApplicationError>>) {
    let (sender, receiver) = mpsc::sync_channel(1);
    let thread = thread::spawn(move || {
        let result = harness.run(limit);
        let _ = sender.send(result);
    });
    (thread, receiver)
}

#[test]
fn held_dispatch_and_worker_calls_share_one_serial_execution_slot() {
    let mut harness = Harness::new(2, vec![], vec![]);
    let state = Arc::clone(&harness.state);
    let (dispatch_gate, dispatch_entered, release_dispatch) = Gate::new();
    let (worker_gate, worker_entered, release_worker) = Gate::new();
    harness.dispatch.gate = Some(dispatch_gate);
    harness.worker.gate = Some(worker_gate);
    let (thread, done) = spawn(harness, 3);

    let dispatch_was_entered = dispatch_entered.recv_timeout(WATCHDOG).is_ok();
    let during_dispatch = state.calls();
    let active_during_dispatch = state.active();
    let worker_entered_early = worker_entered.try_recv().is_ok();
    let returned_during_dispatch = done.try_recv().ok();
    let _ = release_dispatch.send(());

    let worker_was_entered = worker_entered_early || worker_entered.recv_timeout(WATCHDOG).is_ok();
    let during_worker = state.calls();
    let active_during_worker = state.active();
    let returned_while_held = returned_during_dispatch.or_else(|| done.try_recv().ok());
    let premature = returned_while_held.is_some();
    let _ = release_worker.send(());
    let result = returned_while_held.unwrap_or_else(|| {
        done.recv_timeout(WATCHDOG)
            .expect("consumer completion watchdog")
    });
    thread.join().unwrap();

    assert!(dispatch_was_entered, "consumer never called dispatcher");
    assert!(worker_was_entered, "consumer never called worker");
    assert!(
        !worker_entered_early,
        "worker entered before dispatch returned"
    );
    assert!(
        !premature,
        "consumer returned while a port call remained in flight"
    );
    assert_eq!(active_during_dispatch, 1);
    assert_eq!(active_during_worker, 1);
    assert_eq!(during_dispatch, vec![Call::Dispatch(Events, 3)]);
    assert_eq!(during_worker, vec![Call::Dispatch(Events, 3), Call::Worker]);
    result.unwrap();
    let mut expected = cycle(Events, 3, 3);
    expected.extend(cycle(LegacyBootstrap, 3, 3));
    assert_eq!(state.calls(), expected);
    assert_eq!(state.maximum(), 1, "dispatch and worker calls overlapped");
    assert_eq!(state.active(), 0);
}

#[test]
fn stop_waits_for_an_admitted_dispatch_or_worker_and_prevents_the_next_call() {
    for hold_worker in [false, true] {
        let mut harness = Harness::new(1, vec![], vec![]);
        let state = Arc::clone(&harness.state);
        let (gate, entered, release) = Gate::new();
        if hold_worker {
            harness.worker.gate = Some(gate);
        } else {
            harness.dispatch.gate = Some(gate);
        }
        let (thread, done) = spawn(harness, 100);
        let admitted = entered.recv_timeout(WATCHDOG).is_ok();
        state.stop();
        let during_stop = state.calls();
        let active_during_stop = state.active();
        let early = done.try_recv().ok();
        let premature = early.is_some();
        let _ = release.send(());
        let result = early.unwrap_or_else(|| {
            done.recv_timeout(WATCHDOG)
                .expect("stopped consumer watchdog")
        });
        thread.join().unwrap();

        assert!(admitted, "consumer never entered the selected port");
        assert!(!premature, "stop detached the in-flight call");
        assert_eq!(active_during_stop, 1);
        result.unwrap();
        let mut expected = vec![Call::Dispatch(Events, 100)];
        if hold_worker {
            expected.push(Call::Worker);
        }
        assert_eq!(during_stop, expected);
        assert_eq!(
            state.calls(),
            expected,
            "stop admitted another call or wait"
        );
        assert_eq!(state.maximum(), 1);
        assert_eq!(state.active(), 0);
    }
}
