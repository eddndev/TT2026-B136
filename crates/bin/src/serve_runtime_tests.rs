use super::{support::*, ShutdownSignal};

#[tokio::test]
async fn live_http_and_consumer_remain_supervised_until_an_exit_event() {
    let mut running = Running::start(ConsumerExit::Success).await;
    let initially_pending = running.pending();
    let initially_stopped = running.stopped();
    let initially_shutting_down = running.shutdown_seen();
    running.signal(ShutdownSignal::Interrupt);
    running.pending();
    running.release_http(false);
    running.release_consumer();
    running.consumer_done().await;
    let result = running.finish().await;

    assert!(
        initially_pending,
        "supervisor returned without an exit event"
    );
    assert!(!initially_stopped);
    assert!(!initially_shutting_down);
    result.unwrap();
}

#[tokio::test]
async fn interrupt_and_terminate_drain_http_then_join_the_in_flight_consumer() {
    for signal in [ShutdownSignal::Interrupt, ShutdownSignal::Terminate] {
        let mut running = Running::start(ConsumerExit::Success).await;
        let initially_pending = running.pending();
        running.signal(signal);
        let pending_after_signal = running.pending();
        let stopped = running.stopped();
        let http_notified = running.shutdown_seen();
        running.release_http(false);
        let pending_after_http = running.pending();
        let http_drained = running.http_finished();
        let both_drained_early = running.both_finished();
        running.release_consumer();
        running.consumer_done().await;
        let result = running.finish().await;

        assert!(initially_pending);
        assert!(pending_after_signal, "{signal:?} detached active work");
        assert!(stopped, "{signal:?} did not request consumer stop");
        assert!(
            http_notified,
            "{signal:?} did not start graceful HTTP shutdown"
        );
        assert!(
            http_drained,
            "HTTP drainage was not driven while consumer was blocked"
        );
        assert!(
            pending_after_http,
            "supervisor did not retain the consumer join"
        );
        assert!(!both_drained_early);
        result.unwrap();
    }
}

#[tokio::test]
async fn signal_still_waits_for_http_when_the_consumer_finishes_first() {
    let mut running = Running::start(ConsumerExit::Success).await;
    running.pending();
    running.signal(ShutdownSignal::Terminate);
    running.pending();
    running.release_consumer();
    running.consumer_done().await;
    let still_draining = running.pending();
    let http_finished_early = running.http_finished();
    running.release_http(false);
    let result = running.finish().await;

    assert!(still_draining, "consumer completion detached HTTP drainage");
    assert!(!http_finished_early);
    result.unwrap();
}

#[tokio::test]
async fn unexpected_http_completion_or_failure_stops_and_joins_consumer() {
    for (failed, code) in [(false, "http_stopped"), (true, "http_failed")] {
        let mut running = Running::start(ConsumerExit::Success).await;
        running.pending();
        running.release_http(failed);
        let joining = running.pending();
        let stopped = running.stopped();
        let http_drained = running.http_finished();
        running.release_consumer();
        running.consumer_done().await;
        let result = running.finish().await;

        assert!(joining, "HTTP exit detached the in-flight consumer");
        assert!(stopped);
        assert!(http_drained);
        assert_codes(result, &[code]);
    }
}

#[tokio::test]
async fn consumer_error_panic_or_unexpected_success_stops_and_drains_http() {
    for (exit, code) in [
        (ConsumerExit::Error, "consumer_failed"),
        (ConsumerExit::Panic, "consumer_panicked"),
        (ConsumerExit::Success, "consumer_stopped"),
    ] {
        let mut running = Running::start(exit).await;
        running.pending();
        running.release_consumer();
        running.consumer_done().await;
        let draining = running.pending();
        let stopped = running.stopped();
        let notified = running.shutdown_seen();
        let http_finished_early = running.http_finished();
        running.release_http(false);
        let result = running.finish().await;

        assert!(draining, "consumer exit dropped the HTTP future");
        assert!(stopped);
        assert!(
            notified,
            "consumer exit did not request graceful HTTP shutdown"
        );
        assert!(!http_finished_early);
        assert_codes(result, &[code]);
    }
}

#[tokio::test]
async fn signal_failure_stops_and_drains_both_sides_with_a_safe_error() {
    let mut running = Running::start(ConsumerExit::Success).await;
    running.pending();
    running.signal_error();
    let pending = running.pending();
    let stopped = running.stopped();
    let notified = running.shutdown_seen();
    running.release_http(false);
    running.release_consumer();
    running.consumer_done().await;
    let result = running.finish().await;

    assert!(pending);
    assert!(stopped);
    assert!(notified);
    assert_codes(result, &["signal_failed"]);
}

#[tokio::test]
async fn signal_shutdown_preserves_http_and_consumer_failures_during_drain() {
    for (exit, code) in [
        (ConsumerExit::Error, "consumer_failed"),
        (ConsumerExit::Panic, "consumer_panicked"),
    ] {
        let mut running = Running::start(exit).await;
        running.pending();
        running.signal(ShutdownSignal::Interrupt);
        running.pending();
        running.release_http(true);
        let joined_before_release = !running.pending();
        running.release_consumer();
        running.consumer_done().await;
        let result = running.finish().await;

        assert!(
            !joined_before_release,
            "HTTP failure abandoned the consumer join"
        );
        assert_codes(result, &["http_failed", code]);
    }
}

#[tokio::test]
async fn http_failure_preserves_a_consumer_failure_discovered_while_joining() {
    let mut running = Running::start(ConsumerExit::Error).await;
    running.pending();
    running.release_http(true);
    let joining = running.pending();
    running.release_consumer();
    running.consumer_done().await;
    let result = running.finish().await;

    assert!(joining);
    assert_codes(result, &["http_failed", "consumer_failed"]);
}

#[tokio::test]
async fn consumer_failure_preserves_an_http_failure_discovered_during_drain() {
    let mut running = Running::start(ConsumerExit::Error).await;
    running.pending();
    running.release_consumer();
    running.consumer_done().await;
    let draining = running.pending();
    running.release_http(true);
    let result = running.finish().await;

    assert!(draining);
    assert_codes(result, &["consumer_failed", "http_failed"]);
}
