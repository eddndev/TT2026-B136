use super::{run, support::*, AlertRuntimeConfig};
use application::{alerts::*, ApplicationError, PortFailureKind};
use std::{sync::atomic::Ordering::SeqCst, time::Duration};

#[test]
fn budgets_are_bounded_and_disabled_email_never_claims_after_a_scheduler_batch() {
    assert_eq!(AlertRuntimeConfig::default().budget(), 20);
    assert_eq!(AlertRuntimeConfig::default().poll(), Duration::from_secs(1));
    for budget in [0, 101, u32::MAX] {
        assert!(AlertRuntimeConfig::new(budget, Duration::from_secs(1)).is_err());
    }
    assert!(AlertRuntimeConfig::new(1, Duration::ZERO).is_err());
    for budget in [1, 100] {
        assert!(AlertRuntimeConfig::new(budget, Duration::from_millis(1)).is_ok());
    }
    let control = Control::new(1, None);
    let scheduler = Scheduler::new(&control, vec![progress(), progress(), progress()]);
    let delivery = Delivery::new(&control, vec![], vec![]);
    run(&scheduler, &delivery, None, config(2), control.as_ref()).unwrap();
    assert_eq!(control.events(), vec!["schedule", "schedule", "wait"]);
    assert_eq!(scheduler.replies.lock().unwrap().len(), 1);
    assert_eq!(*control.waits.lock().unwrap(), vec![Duration::from_secs(1)]);
}

#[test]
fn uncertain_send_is_completed_with_its_exact_fence_and_retried_without_resending() {
    let control = Control::new(2, None);
    let scheduler = Scheduler::new(&control, vec![Ok(AlertSchedulerRun::Idle)]);
    let expected = claim();
    let delivery = Delivery::new(
        &control,
        vec![Ok(Some(expected.clone()))],
        vec![Err(ApplicationError::Port("temporary".into())), Ok(())],
    );
    let sender = Sender::new(&control);
    run(
        &scheduler,
        &delivery,
        Some(&sender),
        config(20),
        control.as_ref(),
    )
    .unwrap();
    assert_eq!(
        control.events(),
        vec!["schedule", "claim", "send", "complete", "wait", "complete", "wait"]
    );
    assert_eq!(*sender.sent.lock().unwrap(), vec![expected.message]);
    let completions = delivery.completions.lock().unwrap();
    assert_eq!(completions.len(), 2);
    assert_eq!(completions[0], completions[1]);
    assert_eq!(completions[0].delivery_id, expected.delivery_id);
    assert_eq!(completions[0].claim_id, expected.claim_id);
    assert_eq!(completions[0].attempt, expected.attempt);
    assert_eq!(completions[0].outcome, sender.outcome);
}

#[test]
fn stop_boundaries_leave_an_unsent_claim_but_confirm_an_already_known_send_result() {
    for stop_at in ["before", "schedule", "claim", "send"] {
        let control = Control::new(1, Some(stop_at));
        if stop_at == "before" {
            control.stopped.store(true, SeqCst);
        }
        let scheduler = Scheduler::new(&control, vec![Ok(AlertSchedulerRun::Idle)]);
        let delivery = Delivery::new(&control, vec![Ok(Some(claim()))], vec![Ok(())]);
        let sender = Sender::new(&control);
        run(
            &scheduler,
            &delivery,
            Some(&sender),
            config(20),
            control.as_ref(),
        )
        .unwrap();
        let expected = match stop_at {
            "before" => vec![],
            "schedule" => vec!["schedule"],
            "claim" => vec!["schedule", "claim"],
            _ => vec!["schedule", "claim", "send", "complete"],
        };
        assert_eq!(control.events(), expected, "{stop_at}");
    }
}

#[test]
fn transient_failures_pause_before_retry_and_stored_corruption_remains_fatal() {
    for failure in [
        ApplicationError::Port("unavailable".into()),
        ApplicationError::ClassifiedPort {
            kind: PortFailureKind::Busy,
            message: "busy".into(),
        },
    ] {
        let control = Control::new(2, None);
        let scheduler = Scheduler::new(&control, vec![Err(failure), Ok(AlertSchedulerRun::Idle)]);
        let delivery = Delivery::new(&control, vec![], vec![]);
        run(&scheduler, &delivery, None, config(20), control.as_ref()).unwrap();
        assert_eq!(
            control.events(),
            vec!["schedule", "wait", "schedule", "wait"]
        );
    }
    let control = Control::new(1, None);
    let scheduler = Scheduler::new(
        &control,
        vec![Err(AlertError::Stored("corrupt capture".into()).into())],
    );
    let delivery = Delivery::new(&control, vec![], vec![]);
    assert!(matches!(
        run(&scheduler, &delivery, None, config(20), control.as_ref()),
        Err(ApplicationError::Alert(AlertError::Stored(_)))
    ));
    assert_eq!(control.events(), vec!["schedule"]);
}
