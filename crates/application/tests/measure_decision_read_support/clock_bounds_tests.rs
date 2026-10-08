use super::*;
use domain::{crypto::DocumentHasher, DomainError};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

struct SequenceClock(Mutex<Vec<OffsetDateTime>>);
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let mut values = self.0.lock().unwrap();
        if values.len() == 1 {
            values[0]
        } else {
            values.remove(0)
        }
    }
}

#[test]
fn every_read_rejects_future_groups_and_non_utc_or_regressing_clocks() {
    let saved = operation(10);
    let actor = reader(Role::Paralegal);
    let non_utc = now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for kind in READS {
        for observations in [
            vec![at() - Duration::seconds(1); 2],
            vec![now(), now() - Duration::nanoseconds(1)],
            vec![now(), non_utc],
            vec![now(), year_zero],
        ] {
            let store = successful_store(&actor, &saved, saved.clone(), kind);
            let clock = Arc::new(SequenceClock(Mutex::new(observations)));
            assert!(read(
                &service_with_clock(store, identity(&actor), clock),
                kind,
                &saved
            )
            .is_err());
        }
        for started_at in [non_utc, year_zero] {
            assert!(read(
                &service_with_clock(
                    MockReads::new(),
                    identity(&actor),
                    Arc::new(FixedClock(started_at))
                ),
                kind,
                &saved,
            )
            .is_err());
        }
    }
}

struct CountHasher(Arc<AtomicUsize>);
impl DocumentHasher for CountHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_bytes(bytes)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(reader)
    }
}

fn counted(store: MockReads) -> (MeasureDecisionReadService, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let service = MeasureDecisionReadService::new(
        Arc::new(store),
        Arc::new(identity(&reader(Role::Paralegal))),
        Arc::new(CountHasher(count.clone())),
        Arc::new(FixedClock(now())),
    );
    (service, count)
}

#[test]
fn oversized_page_is_rejected_before_verifying_any_group() {
    let saved = operation(10);
    let case_id = saved.group.review.case_id;
    let returned = page(case_id, vec![saved; 11]);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let (service, count) = counted(store);
    assert!(service
        .list("session", case_id, MeasureDecisionReadQuery::default())
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn oversized_group_or_ancestor_collection_is_rejected_before_hash_verification() {
    let saved = operation(10);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        for mutation in 0..2 {
            let mut returned = saved.clone();
            if mutation == 0 {
                returned.group.measures = vec![saved.group.measures[0].clone(); 33];
            } else {
                returned.measure_history.groups = vec![
                    MeasureGroupEvidence {
                        origin: saved.origin.clone(),
                        capture: saved.group.clone(),
                    };
                    256
                ];
            }
            let store = successful_store(&actor, &saved, returned, kind);
            let (service, count) = counted(store);
            assert!(read(&service, kind, &saved).is_err());
            assert_eq!(count.load(Ordering::SeqCst), 0);
        }
    }
}
