use super::{support::*, DeadlineRuntimeConfig};
use application::{
    deadline_dispatch::{DeadlineDispatchLimit, DeadlineDispatchStream::*},
    deadlines::DeadlineError,
    ApplicationError, PortFailureKind,
};
use std::time::Duration;

#[test]
fn configuration_rejects_zero_pause_and_preserves_validated_values() {
    for number in [1, 20, 100] {
        let limit = DeadlineDispatchLimit::new(number).unwrap();
        assert!(DeadlineRuntimeConfig::new(limit, Duration::ZERO).is_err());
        for poll in [Duration::from_nanos(1), POLL, Duration::from_secs(60)] {
            let config = DeadlineRuntimeConfig::new(limit, poll).unwrap();
            assert_eq!(config.limit(), limit);
            assert_eq!(config.poll(), poll);
        }
    }
}

#[test]
fn busy_cycles_alternate_streams_and_bound_worker_calls_at_both_limits() {
    for limit in [1, 100] {
        let harness = Harness::new(3, vec![], vec![]);
        harness.run(limit).unwrap();
        let expected = [Events, LegacyBootstrap, Events]
            .into_iter()
            .flat_map(|stream| cycle(stream, limit, limit as usize))
            .collect::<Vec<_>>();
        assert_eq!(harness.state.calls(), expected, "limit {limit}");
        assert_eq!(harness.state.maximum(), 1);
        assert_eq!(harness.state.active(), 0);
    }
}

#[test]
fn completed_and_both_durable_deferrals_consume_budget_without_an_extra_wait() {
    let harness = Harness::new(
        2,
        vec![],
        vec![
            WorkerStep::Completed,
            WorkerStep::Deferred(false),
            WorkerStep::Deferred(true),
            WorkerStep::Idle,
        ],
    );
    harness.run(5).unwrap();
    let mut expected = cycle(Events, 5, 4);
    expected.extend(cycle(LegacyBootstrap, 5, 5));
    assert_eq!(harness.state.calls(), expected);
}

#[test]
fn empty_dispatch_and_idle_worker_still_pause_and_alternate() {
    let harness = Harness::new(
        2,
        vec![DispatchStep::Empty, DispatchStep::Empty],
        vec![WorkerStep::Idle, WorkerStep::Idle],
    );
    harness.run(100).unwrap();
    let mut expected = cycle(Events, 100, 1);
    expected.extend(cycle(LegacyBootstrap, 100, 1));
    assert_eq!(harness.state.calls(), expected);
}

fn recoverable_failures() -> [Failure; 4] {
    [
        Failure::Port,
        Failure::Classified(PortFailureKind::Busy),
        Failure::Classified(PortFailureKind::Interrupted),
        Failure::Classified(PortFailureKind::Unavailable),
    ]
}

#[test]
fn dispatch_port_errors_allow_queued_work_and_retry_only_on_the_next_cycle() {
    for failure in recoverable_failures() {
        let harness = Harness::new(
            2,
            vec![DispatchStep::Fail(failure), DispatchStep::Fail(failure)],
            vec![WorkerStep::Idle, WorkerStep::Idle],
        );
        harness.run(20).unwrap();
        let mut expected = cycle(Events, 20, 1);
        expected.extend(cycle(LegacyBootstrap, 20, 1));
        assert_eq!(harness.state.calls(), expected, "{failure:?}");
    }
}

#[test]
fn worker_port_errors_end_the_batch_and_retry_after_its_pause() {
    for failure in recoverable_failures() {
        let harness = Harness::new(
            2,
            vec![],
            vec![
                WorkerStep::Fail(failure),
                WorkerStep::Completed,
                WorkerStep::Idle,
            ],
        );
        harness.run(20).unwrap();
        let mut expected = cycle(Events, 20, 1);
        expected.extend(cycle(LegacyBootstrap, 20, 2));
        assert_eq!(harness.state.calls(), expected, "{failure:?}");
    }
}

#[test]
fn dispatcher_integrity_and_configuration_failures_stop_before_worker_execution() {
    for failure in [Failure::Integrity, Failure::Configuration] {
        let harness = Harness::new(1, vec![DispatchStep::Fail(failure)], vec![]);
        let error = harness.run(20).unwrap_err();
        assert_fatal(error, failure);
        assert_eq!(harness.state.calls(), vec![Call::Dispatch(Events, 20)]);
    }
}

#[test]
fn worker_integrity_and_configuration_failures_stop_before_another_job() {
    for failure in [Failure::Integrity, Failure::Configuration] {
        let harness = Harness::new(1, vec![], vec![WorkerStep::Fail(failure)]);
        let error = harness.run(20).unwrap_err();
        assert_fatal(error, failure);
        assert_eq!(
            harness.state.calls(),
            vec![Call::Dispatch(Events, 20), Call::Worker]
        );
    }
}

fn assert_fatal(error: ApplicationError, expected: Failure) {
    match (expected, error) {
        (
            Failure::Integrity,
            ApplicationError::Deadline(DeadlineError::StoredInconsistent(value)),
        ) => {
            assert_eq!(value, "scripted evidence");
        }
        (Failure::Configuration, ApplicationError::InvalidConfiguration(value)) => {
            assert_eq!(value, "scripted inventory");
        }
        (expected, actual) => panic!("expected {expected:?}, got {actual:?}"),
    }
}

#[test]
fn stop_before_start_admits_no_port_or_wait_calls() {
    let harness = Harness::new(1, vec![], vec![]);
    harness.state.stop();
    harness.run(100).unwrap();
    assert!(harness.state.calls().is_empty());
}

#[test]
fn stop_after_dispatch_prevents_worker_and_pause_even_on_a_recoverable_error() {
    for step in [DispatchStep::Page, DispatchStep::Fail(Failure::Port)] {
        let harness = Harness::new(1, vec![step], vec![]);
        harness.state.stop_after_dispatch(1);
        harness.run(100).unwrap();
        assert_eq!(harness.state.calls(), vec![Call::Dispatch(Events, 100)]);
    }
}

#[test]
fn stop_after_each_worker_return_prevents_the_next_job_or_cycle() {
    for step in [
        WorkerStep::Completed,
        WorkerStep::Deferred(false),
        WorkerStep::Deferred(true),
        WorkerStep::Idle,
        WorkerStep::Fail(Failure::Port),
    ] {
        let harness = Harness::new(1, vec![], vec![WorkerStep::Completed, step]);
        harness.state.stop_after_worker(2);
        harness.run(100).unwrap();
        assert_eq!(
            harness.state.calls(),
            vec![Call::Dispatch(Events, 100), Call::Worker, Call::Worker]
        );
    }
}
