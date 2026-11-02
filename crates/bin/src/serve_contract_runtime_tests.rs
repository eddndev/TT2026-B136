use super::*;
use crate::{serve_alert_supervisor::supervise_with_contracts, serve_stop::Stop};
use application::contracts::ContractNoticeState;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Mutex,
    },
};

struct Control {
    remaining: AtomicUsize,
    waits: AtomicUsize,
}
impl Control {
    fn new(remaining: usize) -> Self {
        Self {
            remaining: AtomicUsize::new(remaining),
            waits: AtomicUsize::new(0),
        }
    }
}
impl RuntimeControl for Control {
    fn is_stopped(&self) -> bool {
        self.remaining.load(SeqCst) == 0
    }
    fn wait(&self, duration: Duration) {
        assert_eq!(duration, CONTRACT_POLL);
        self.remaining.fetch_sub(1, SeqCst);
        self.waits.fetch_add(1, SeqCst);
    }
}

fn expiry(values: Vec<Result<usize, ApplicationError>>) -> (ExpiryCycle, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let values = Mutex::new(VecDeque::from(values));
    let cycle: ExpiryCycle = Arc::new(move || {
        counter.fetch_add(1, SeqCst);
        values.lock().unwrap().pop_front().unwrap_or(Ok(0))
    });
    (cycle, calls)
}

fn notices(
    values: Vec<Result<ContractNoticeRun, ApplicationError>>,
) -> (NoticeAttempt, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let values = Mutex::new(VecDeque::from(values));
    let attempt: NoticeAttempt = Arc::new(move || {
        counter.fetch_add(1, SeqCst);
        values
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(ContractNoticeRun::Recorded(ContractNoticeState::Sent)))
    });
    (attempt, calls)
}

#[test]
fn the_closer_runs_every_cycle_without_email_and_never_after_stop() {
    let (cycle, calls) = expiry(vec![Ok(3), Err(ApplicationError::Port("private".into()))]);
    let consumer = ContractConsumer {
        expiry: cycle,
        notices: None,
    };
    run(&consumer, &Control::new(0)).unwrap();
    assert_eq!(calls.load(SeqCst), 0);
    let control = Control::new(3);
    run(&consumer, &control).unwrap();
    assert_eq!(calls.load(SeqCst), 3);
    assert_eq!(control.waits.load(SeqCst), 3);
}

#[test]
fn enabled_email_sends_at_most_one_batch_per_cycle_until_idle() {
    let (cycle, _) = expiry(Vec::new());
    let (attempt, calls) = notices(Vec::new());
    let consumer = ContractConsumer {
        expiry: cycle.clone(),
        notices: Some(attempt),
    };
    run(&consumer, &Control::new(1)).unwrap();
    assert_eq!(calls.load(SeqCst), NOTICE_BATCH);
    let (attempt, calls) = notices(vec![
        Ok(ContractNoticeRun::Stale),
        Ok(ContractNoticeRun::Idle),
        Err(ApplicationError::Port("private provider".into())),
    ]);
    let consumer = ContractConsumer {
        expiry: cycle,
        notices: Some(attempt),
    };
    run(&consumer, &Control::new(2)).unwrap();
    assert_eq!(calls.load(SeqCst), 3);
}

#[test]
fn a_permanent_failure_of_either_task_ends_the_consumer() {
    let (cycle, _) = expiry(vec![Err(ApplicationError::InvalidConfiguration(
        "private".into(),
    ))]);
    let consumer = ContractConsumer {
        expiry: cycle,
        notices: None,
    };
    let control = Control::new(5);
    assert!(run(&consumer, &control).is_err());
    assert_eq!(control.waits.load(SeqCst), 0);
    let (cycle, _) = expiry(Vec::new());
    let (attempt, _) = notices(vec![Err(ApplicationError::PermissionDenied)]);
    let consumer = ContractConsumer {
        expiry: cycle,
        notices: Some(attempt),
    };
    assert!(run(&consumer, &Control::new(5)).is_err());
}

fn wait_for_stop(
    stop: Arc<Stop>,
    done: Arc<AtomicUsize>,
) -> tokio::task::JoinHandle<Result<(), ApplicationError>> {
    tokio::spawn(async move {
        stop.stopped().await;
        done.fetch_add(1, SeqCst);
        Ok(())
    })
}

#[tokio::test]
async fn a_contract_consumer_exit_stops_and_joins_every_other_consumer() {
    for (result, code) in [
        (Ok(()), "contracts_stopped"),
        (
            Err(ApplicationError::Port("private storage".into())),
            "contracts_failed",
        ),
    ] {
        let stop = Arc::new(Stop::default());
        let done = Arc::new(AtomicUsize::new(0));
        let worker = || wait_for_stop(stop.clone(), done.clone());
        let contracts = tokio::spawn(async move { result });
        let value = tokio::time::timeout(
            Duration::from_secs(3),
            supervise_with_contracts(
                worker(),
                worker(),
                worker(),
                Some(worker()),
                Some(worker()),
                Some(worker()),
                Some(contracts),
                stop.clone(),
            ),
        )
        .await
        .unwrap();
        assert!(stop.is_stopped());
        assert_eq!(done.load(SeqCst), 6);
        let error = value.unwrap_err().to_string();
        assert!(error.contains(code), "{error}");
        assert!(!error.contains("private"));
    }
}

#[tokio::test]
async fn a_requested_stop_joins_the_contract_consumer_without_failure() {
    let stop = Arc::new(Stop::default());
    stop.request();
    let done = Arc::new(AtomicUsize::new(0));
    let worker = || wait_for_stop(stop.clone(), done.clone());
    tokio::time::timeout(
        Duration::from_secs(3),
        supervise_with_contracts(
            worker(),
            worker(),
            worker(),
            None,
            None,
            None,
            Some(worker()),
            stop.clone(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(done.load(SeqCst), 4);
}
