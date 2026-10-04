use super::*;
use domain::clock::{Clock, OffsetDateTime};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use time::Duration;

#[test]
fn fresh_capture_cannot_predate_the_precommit_service_observation() {
    let fixture = Fixture::schedule();
    let review = fixture.review();
    let expected = confirmation(&review);
    let actor = fixture.actor.clone();
    let case_id = fixture.case_id;
    let material = fixture.material.clone();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |current, case, prepared| {
            assert_eq!(current, &actor);
            assert_eq!(case, case_id);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.review(), &review);
            assert_eq!(prepared.material(), &material);
            let result = prepared.into_operation(at() + Duration::seconds(99));
            assert!(
                result.is_err(),
                "fresh capture accepted a timestamp before the precommit observation"
            );
            result
        });
    let harness = harness_with(
        store,
        identity(fixture.actor),
        Validator::default(),
        Arc::new(FixedClock(at() + Duration::seconds(100))),
    );
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
    assert_eq!(harness.validator.calls(), 1);
}

struct ThreeObservationClock {
    calls: AtomicUsize,
}

impl Clock for ThreeObservationClock {
    fn now(&self) -> OffsetDateTime {
        let seconds = match self.calls.fetch_add(1, Ordering::SeqCst) {
            0 => 100,
            1 => 110,
            _ => 105,
        };
        at() + Duration::seconds(seconds)
    }
}

#[test]
fn postcommit_clock_cannot_regress_from_the_precommit_observation_on_raced_replay() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at() + Duration::seconds(99));
    let expected = confirmation(&original.capture.review);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.review(), &original.capture.review);
            Ok(original)
        });
    let clock = Arc::new(ThreeObservationClock {
        calls: AtomicUsize::new(0),
    });
    let harness = harness_with(
        store,
        identity(fixture.actor),
        Validator::default(),
        clock.clone(),
    );
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
    assert_eq!(clock.calls.load(Ordering::SeqCst), 3);
}
