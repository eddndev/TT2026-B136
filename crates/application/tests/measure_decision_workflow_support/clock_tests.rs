use super::*;
use domain::clock::{Clock, OffsetDateTime};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use time::UtcOffset;

struct SequenceClock {
    observations: Vec<OffsetDateTime>,
    calls: AtomicUsize,
}
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.observations[index.min(self.observations.len() - 1)]
    }
}

fn clock(observations: Vec<OffsetDateTime>) -> Arc<dyn Clock + Send + Sync> {
    Arc::new(SequenceClock {
        observations,
        calls: AtomicUsize::new(0),
    })
}

fn assert_precommit_denial(
    fixture: Fixture,
    clock: Arc<dyn Clock + Send + Sync>,
    commit_at: OffsetDateTime,
) {
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    let commits = Arc::new(AtomicUsize::new(0));
    let count = commits.clone();
    store
        .expect_commit()
        .times(0..=1)
        .return_once(move |_, _, prepared| {
            count.fetch_add(1, Ordering::SeqCst);
            prepared.into_operation(commit_at)
        });
    let harness = harness_with(store, identity(fixture.actor), Validator::default(), clock);
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
    assert_eq!(commits.load(Ordering::SeqCst), 0);
}

#[test]
fn future_source_provenance_cannot_reach_commit() {
    let mut fixture = Fixture::single();
    fixture.material.result_sources[0]
        .sources
        .subject
        .changed_at = now() + Duration::seconds(5);
    fixture.operation(now() + Duration::seconds(6));
    assert_precommit_denial(fixture, clock(vec![now()]), now() + Duration::seconds(6));
}

#[test]
fn regressing_or_non_utc_precommit_observations_cannot_reach_commit() {
    for second in [
        now() - Duration::nanoseconds(1),
        now().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
    ] {
        assert_precommit_denial(Fixture::single(), clock(vec![now(), second]), now());
    }
}

#[test]
fn a_fresh_prepared_operation_cannot_capture_before_its_precommit_clock_floor() {
    let fixture = Fixture::single();
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
fn final_clock_compares_against_precommit_observation_even_for_an_older_raced_replay() {
    let fixture = Fixture::single();
    let original = fixture.operation(at());
    let expected = confirmation(&original.group.review);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(original));
    let harness = harness_with(
        store,
        identity(fixture.actor),
        Validator::default(),
        clock(vec![
            now(),
            now() + Duration::seconds(2),
            now() + Duration::seconds(1),
        ]),
    );
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}
