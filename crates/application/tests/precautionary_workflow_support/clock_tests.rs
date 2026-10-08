use super::*;
use application::typed_participants::ParticipantRevisionSnapshot;
use domain::clock::{Clock, OffsetDateTime};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use time::{Duration, UtcOffset};

struct ChangingClock {
    first: OffsetDateTime,
    subsequent: OffsetDateTime,
    calls: AtomicUsize,
}

impl Clock for ChangingClock {
    fn now(&self) -> OffsetDateTime {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first
        } else {
            self.subsequent
        }
    }
}

fn assert_denied_before_commit(
    fixture: Fixture,
    clock: Arc<dyn Clock + Send + Sync>,
    commit_at: OffsetDateTime,
) {
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    let commits = Arc::new(AtomicUsize::new(0));
    let observed = commits.clone();
    store
        .expect_commit()
        .times(0..=1)
        .return_once(move |_, _, prepared| {
            observed.fetch_add(1, Ordering::SeqCst);
            prepared.into_operation(commit_at)
        });
    let harness = harness_with(store, identity(fixture.actor), Validator::default(), clock);
    let result = harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected);
    assert!(result.is_err());
    assert_eq!(
        commits.load(Ordering::SeqCst),
        0,
        "invalid service chronology reached commit"
    );
}

#[test]
fn a_future_source_capture_is_rejected_before_commit_even_when_its_receipt_is_valid() {
    let mut fixture = Fixture::schedule();
    let sources = fixture.material.selected_sources.as_mut().unwrap();
    let ParticipantRevisionSnapshot::Manual(source) = &mut sources.participants[0].revision else {
        unreachable!()
    };
    source.changed_at = at() + Duration::seconds(5);
    let commit_at = at() + Duration::seconds(6);
    fixture.operation(commit_at);
    assert_denied_before_commit(fixture, Arc::new(FixedClock(at())), commit_at);
}

#[test]
fn a_service_clock_regression_after_admission_is_rejected_before_commit() {
    let first = at() + Duration::seconds(10);
    assert_denied_before_commit(
        Fixture::schedule(),
        Arc::new(ChangingClock {
            first,
            subsequent: first - Duration::nanoseconds(1),
            calls: AtomicUsize::new(0),
        }),
        first,
    );
}

#[test]
fn a_non_utc_service_clock_after_admission_is_rejected_before_commit() {
    let first = at() + Duration::seconds(10);
    assert_denied_before_commit(
        Fixture::schedule(),
        Arc::new(ChangingClock {
            first,
            subsequent: first.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
            calls: AtomicUsize::new(0),
        }),
        first,
    );
}
