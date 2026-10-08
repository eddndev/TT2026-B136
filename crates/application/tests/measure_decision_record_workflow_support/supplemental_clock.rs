use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct SequenceClock {
    times: Vec<OffsetDateTime>,
    calls: AtomicUsize,
}
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let at = self.calls.fetch_add(1, Ordering::SeqCst);
        self.times[at.min(self.times.len() - 1)]
    }
}

fn clock(times: Vec<OffsetDateTime>) -> Arc<dyn Clock + Send + Sync> {
    Arc::new(SequenceClock {
        times,
        calls: AtomicUsize::new(0),
    })
}

#[test]
fn unsupported_start_clock_and_regressing_precommit_clock_reject_without_commit() {
    for at in [
        now().replace_year(0).unwrap(),
        now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap()),
    ] {
        let fixture = Fixture::corrected();
        let (service, observations) = custom_service(
            MockStore::new(),
            identity(fixture.actor),
            Arc::new(Validator::default()),
            clock(vec![at]),
            Arc::new(Hasher),
        );
        assert!(service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(observations.lock().unwrap().events.is_empty());
    }
    let fixture = Fixture::corrected();
    let expected = confirmation(&fixture.review());
    let (service, _) = custom_service(
        fixture.store(),
        identity(fixture.actor),
        Arc::new(Validator::default()),
        clock(vec![now(), now() - Duration::nanoseconds(1)]),
        Arc::new(Hasher),
    );
    assert!(service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

#[test]
fn fresh_prepared_capture_cannot_precede_its_observed_precommit_floor() {
    let fixture = Fixture::corrected();
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now() - Duration::nanoseconds(1)));
    let harness = harness(store, identity(fixture.actor));
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

#[test]
fn future_effective_corrected_record_cannot_be_prepared_or_submitted_yet() {
    for submitting in [false, true] {
        let mut correction = crate::record_support::RecordFixture::initial();
        correction.recorded_at = now() + Duration::seconds(1);
        let administrative = correction.capture();
        let fixture = from_pure(
            crate::record_decision_support::FixtureV2::confirm(
                &administrative,
                &correction.history,
            ),
            96,
        );
        let expected = confirmation(&fixture.review());
        let harness = harness(fixture.store(), identity(fixture.actor));
        let result = if submitting {
            harness
                .service
                .submit("session", fixture.case_id, fixture.command, expected)
                .map(|_| ())
        } else {
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command)
                .map(|_| ())
        };
        assert!(result.is_err());
    }
}

#[test]
fn an_old_raced_capture_does_not_erase_the_latest_clock_observation() {
    let fixture = Fixture::corrected();
    let expected = confirmation(&fixture.review());
    let original = fixture.operation(now() - Duration::seconds(1));
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(original));
    let (service, _) = custom_service(
        store,
        identity(fixture.actor),
        Arc::new(Validator::default()),
        clock(vec![
            now(),
            now() + Duration::seconds(2),
            now() + Duration::seconds(1),
        ]),
        Arc::new(Hasher),
    );
    assert!(service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}
