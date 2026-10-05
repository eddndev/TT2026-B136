use super::*;
use domain::clock::Clock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use time::UtcOffset;

struct SequenceClock {
    values: Vec<OffsetDateTime>,
    calls: AtomicUsize,
}
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.values[index.min(self.values.len() - 1)]
    }
}

#[test]
fn invalid_initial_or_regressing_precommit_clocks_never_reach_commit() {
    for before_store in [false, true] {
        for non_utc in [false, true] {
            let fixture = Fixture::schedule();
            let expected = confirmation(&fixture.review());
            let bad = if non_utc {
                now().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            } else {
                now() - Duration::nanoseconds(1)
            };
            let values = if before_store {
                vec![if non_utc {
                    bad
                } else {
                    now().replace_year(0).unwrap()
                }]
            } else {
                vec![now(), bad]
            };
            let store = if before_store {
                MockStore::new()
            } else {
                fixture.store()
            };
            let h = harness_with(
                store,
                identity(fixture.actor),
                Validator::default(),
                Arc::new(SequenceClock {
                    values,
                    calls: AtomicUsize::new(0),
                }),
            );
            assert!(h
                .service
                .submit("session", fixture.case_id, fixture.command, expected)
                .is_err());
        }
    }
}

#[test]
fn a_valid_future_source_still_blocks_commit_until_the_service_clock_reaches_it() {
    let mut fixture = Fixture::schedule();
    let participants = &mut fixture
        .material
        .selected_sources
        .as_mut()
        .unwrap()
        .participants;
    crate::participant_support::manual_mut(&mut participants[0]).changed_at =
        now() + Duration::seconds(1);
    fixture.operation(now() + Duration::seconds(1));
    let expected = confirmation(&fixture.review());
    let h = harness(fixture.store(), identity(fixture.actor));
    assert!(h
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

#[test]
fn opaque_fresh_capture_rejects_a_timestamp_before_the_precommit_observation() {
    let fixture = Fixture::schedule();
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| {
            let result = prepared.into_operation(now() - Duration::nanoseconds(1));
            assert!(result.is_err());
            result
        });
    let h = harness(store, identity(fixture.actor));
    assert!(h
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

#[test]
fn raced_replay_does_not_relax_postcommit_service_clock_monotonicity() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let expected = confirmation(&original.capture.review);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(original));
    let h = harness_with(
        store,
        identity(fixture.actor),
        Validator::default(),
        Arc::new(SequenceClock {
            values: vec![
                now(),
                now() + Duration::seconds(2),
                now() + Duration::seconds(1),
            ],
            calls: AtomicUsize::new(0),
        }),
    );
    assert!(h
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}
