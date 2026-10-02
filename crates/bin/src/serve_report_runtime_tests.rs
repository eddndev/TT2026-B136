use super::*;
use application::case_reports::{CaseReportError, CaseReportId};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
    time::Duration,
};
struct Control {
    remaining: AtomicUsize,
    waits: AtomicUsize,
}
impl Control {
    fn new(count: usize) -> Self {
        Self {
            remaining: AtomicUsize::new(count),
            waits: AtomicUsize::new(0),
        }
    }
}
impl RuntimeControl for Control {
    fn is_stopped(&self) -> bool {
        self.remaining.load(SeqCst) == 0
    }
    fn wait(&self, duration: Duration) {
        assert_eq!(duration, Duration::from_secs(1));
        self.waits.fetch_add(1, SeqCst);
        self.remaining.fetch_sub(1, SeqCst);
    }
}
#[test]
fn stopped_report_runtime_never_claims_another_job() {
    run(
        &|| panic!("stopped report runtime must not claim"),
        &Control::new(0),
    )
    .unwrap();
}
#[test]
fn report_runtime_pauses_after_each_completed_or_idle_job_without_busy_looping() {
    let results = RefCell::new(VecDeque::from([
        CaseReportWorkerRun::Ready(CaseReportId::new()),
        CaseReportWorkerRun::Idle,
    ]));
    let control = Control::new(2);
    run(&|| Ok(results.borrow_mut().pop_front().unwrap()), &control).unwrap();
    assert!(results.borrow().is_empty());
    assert_eq!(control.waits.load(SeqCst), 2);
}
#[test]
fn report_runtime_recovers_port_errors_and_lost_leases_after_a_pause() {
    let results = RefCell::new(VecDeque::from([
        Err(ApplicationError::Port("temporary test error".into())),
        Err(CaseReportError::LeaseLost.into()),
        Ok(CaseReportWorkerRun::Idle),
    ]));
    let control = Control::new(3);
    run(&|| results.borrow_mut().pop_front().unwrap(), &control).unwrap();
    assert!(results.borrow().is_empty());
    assert_eq!(control.waits.load(SeqCst), 3);
}
#[test]
fn report_runtime_propagates_invalid_configuration_for_supervised_shutdown() {
    let control = Control::new(1);
    assert!(matches!(
        run(
            &|| Err(ApplicationError::InvalidConfiguration(
                "test failure".into()
            )),
            &control
        ),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_eq!(control.waits.load(SeqCst), 0);
}
