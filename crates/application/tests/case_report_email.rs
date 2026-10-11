use application::case_reports::*;
use time::Duration;
use uuid::Uuid;

#[path = "case_report_support/email.rs"]
mod support;
use support::{harness, template, worker};

#[test]
fn empty_queue_does_not_authorize_send_or_finish() {
    let (worker, state) = harness();
    state.lock().unwrap().available = None;
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Idle);
    assert_eq!(state.lock().unwrap().events, ["claim"]);
}

#[test]
fn authorization_precedes_one_send_and_each_provider_outcome_is_fenced() {
    for (outcome, expected) in [
        (
            CaseReportEmailOutcome::Accepted,
            CaseReportEmailState::Accepted,
        ),
        (
            CaseReportEmailOutcome::DefinitelyRejected,
            CaseReportEmailState::Rejected,
        ),
        (
            CaseReportEmailOutcome::Uncertain,
            CaseReportEmailState::Uncertain,
        ),
    ] {
        let (worker, state) = harness();
        state.lock().unwrap().outcome = outcome;
        assert_eq!(
            worker.run_once().unwrap(),
            CaseReportEmailRun::Recorded(expected)
        );
        let state = state.lock().unwrap();
        assert_eq!(state.events, ["claim", "authorize", "deliver", "finish"]);
        let lease = state.claimed.as_ref().unwrap();
        assert_eq!(
            state.delivered,
            [(lease.delivery_id, lease.message.clone())]
        );
        assert_eq!(state.finishes[0].state, expected);
        assert_eq!(
            state.finishes[0].retry_at,
            if expected == CaseReportEmailState::Uncertain {
                Some(state.now + Duration::seconds(60))
            } else {
                None
            }
        );
    }
}

#[test]
fn revocation_or_stale_authorization_prevents_delivery() {
    let (worker, state) = harness();
    state.lock().unwrap().authorized = false;
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Stale);
    assert_eq!(state.lock().unwrap().events, ["claim", "authorize"]);
}

#[test]
fn elapsed_lease_or_provider_window_prevents_authorization_and_delivery() {
    for provider in [false, true] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            let now = state.now;
            let lease = state.available.as_mut().unwrap();
            if provider {
                lease.provider_deadline = now;
            } else {
                lease.lease_until = now;
            }
        }
        assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Stale);
        assert_eq!(state.lock().unwrap().events, ["claim"]);
    }
}

#[test]
fn authorization_cannot_extend_an_elapsed_lease() {
    let (worker, state) = harness();
    state.lock().unwrap().authorize_delay = Duration::seconds(30);
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Stale);
    assert_eq!(state.lock().unwrap().events, ["claim", "authorize"]);
}

#[test]
fn authorization_cannot_substitute_any_persisted_message_field() {
    for field in ["from", "to", "subject", "text"] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            let mut changed = state.available.as_ref().unwrap().message.clone();
            match field {
                "from" => changed.from = "other@example.test".into(),
                "to" => changed.to = "other@example.test".into(),
                "subject" => changed.subject = "changed subject".into(),
                _ => changed.text = "changed text".into(),
            }
            state.changed_message = Some(changed);
        }
        assert!(worker.run_once().is_err());
        assert_eq!(state.lock().unwrap().events, ["claim", "authorize"]);
    }
}

#[test]
fn invalid_or_foreign_lease_identity_is_not_used_for_provider_io() {
    for field in ["report", "delivery", "generation", "overflow", "lease"] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            let lease = state.available.as_mut().unwrap();
            match field {
                "report" => lease.report_id = CaseReportId::from_uuid(Uuid::nil()),
                "delivery" => lease.delivery_id = Uuid::nil(),
                "generation" => lease.generation = 0,
                "overflow" => lease.generation = u64::MAX,
                _ => {
                    lease.lease_id = Uuid::from_u128(99);
                    state.wrong_lease = true;
                }
            }
        }
        assert!(worker.run_once().is_err());
        assert_eq!(state.lock().unwrap().events, ["claim"]);
    }
}

#[test]
fn transport_error_records_uncertainty_without_private_diagnostics_or_inline_retry() {
    let (worker, state) = harness();
    state.lock().unwrap().fail_at = Some("deliver");
    let result = worker.run_once().unwrap();
    assert_eq!(
        result,
        CaseReportEmailRun::Recorded(CaseReportEmailState::Uncertain)
    );
    assert!(!format!("{result:?}").contains("private-"));
    let state = state.lock().unwrap();
    assert_eq!(state.delivered.len(), 1);
    assert_eq!(
        state.finishes[0].retry_at,
        Some(state.now + Duration::seconds(60))
    );
}

#[test]
fn uncertain_retry_must_be_strictly_before_original_provider_deadline() {
    for seconds in [59, 60, 61] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            state.outcome = CaseReportEmailOutcome::Uncertain;
            let deadline = state.now + Duration::seconds(seconds);
            state.available.as_mut().unwrap().provider_deadline = deadline;
        }
        worker.run_once().unwrap();
        let state = state.lock().unwrap();
        assert_eq!(
            state.finishes[0].retry_at,
            (seconds == 61).then_some(state.now + Duration::seconds(60))
        );
    }
}

#[test]
fn stale_finish_cannot_confirm_recording_and_is_not_automatically_retried() {
    let (worker, state) = harness();
    state.lock().unwrap().accepts_finish = false;
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Stale);
    assert_eq!(
        state.lock().unwrap().events,
        ["claim", "authorize", "deliver", "finish"]
    );
}

#[test]
fn store_errors_remain_sanitized_errors_without_automatic_retries() {
    for stage in ["claim", "authorize", "finish"] {
        let (worker, state) = harness();
        state.lock().unwrap().fail_at = Some(stage);
        let error = worker.run_once().unwrap_err();
        assert!(!format!("{error:?} {error}").contains("private-"));
        let state = state.lock().unwrap();
        assert_eq!(
            state.events.iter().filter(|event| **event == stage).count(),
            1
        );
        assert_eq!(state.delivered.len(), usize::from(stage == "finish"));
    }
}

#[test]
fn restarted_worker_reuses_stored_message_and_delivery_identity_after_uncertain_result() {
    let (first, state) = harness();
    state.lock().unwrap().outcome = CaseReportEmailOutcome::Uncertain;
    first.run_once().unwrap();
    {
        let mut state = state.lock().unwrap();
        state.now += Duration::seconds(60);
        let until = state.now + Duration::seconds(30);
        let lease = state.available.as_mut().unwrap();
        lease.generation += 1;
        lease.lease_until = until;
    }
    let mut changed = template();
    changed.from = "changed@example.test".into();
    changed.text = "New origin: https://new.example.test/#reports".into();
    worker(state.clone(), changed).run_once().unwrap();
    let state = state.lock().unwrap();
    assert_eq!(state.delivered.len(), 2);
    assert_eq!(state.delivered[0], state.delivered[1]);
    assert_ne!(state.templates[0], state.templates[1]);
    assert_ne!(state.finishes[0].lease_id, state.finishes[1].lease_id);
    assert_eq!(
        state.finishes[1].generation,
        state.finishes[0].generation + 1
    );
}

#[test]
fn backwards_clock_during_authorization_prevents_delivery() {
    let (worker, state) = harness();
    state.lock().unwrap().authorize_delay = Duration::seconds(-1);
    assert!(worker.run_once().is_err());
    assert_eq!(state.lock().unwrap().events, ["claim", "authorize"]);
}
