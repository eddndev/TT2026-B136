use super::*;
use domain::{crypto::DocumentHasher, DomainError};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

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
fn counted(store: MockReads) -> (MeasureAdministrativeReadService, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    (
        MeasureAdministrativeReadService::new(
            Arc::new(store),
            Arc::new(identity(&reader(Role::Paralegal))),
            Arc::new(CountHasher(calls.clone())),
            Arc::new(FixedClock(now())),
        ),
        calls,
    )
}

#[test]
fn oversized_pages_are_rejected_before_hashing_any_receipt() {
    let original = operation(10);
    let case = original.origin.case_id;
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case, vec![original; 21])));
    let (service, count) = counted(store);
    assert!(service
        .list(
            "session",
            case,
            MeasureAdministrativeReadQuery::new(20, None).unwrap()
        )
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn per_receipt_owner_row_and_nested_group_bounds_are_enforced_before_hashing() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        for mutation in 0..3 {
            let mut returned = original.clone();
            match mutation {
                0 => returned.capture.records = vec![original.capture.records[0].clone(); 2],
                1 => {
                    returned.record_history.records.judicial.groups =
                        vec![original.record_history.records.judicial.groups[0].clone(); 256]
                }
                _ => {
                    let group = &mut returned.record_history.records.judicial.groups[0].capture;
                    group.measures = vec![group.measures[0].clone(); 33];
                }
            }
            let (service, count) = counted(successful_store(&actor, &original, returned, kind));
            assert!(read(&service, kind, &original).is_err());
            assert_eq!(count.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn a_valid_page_does_not_acquire_a_new_aggregate_owner_budget() {
    let mut items = Vec::new();
    for serial in 1..=20 {
        let mut group = root_fixture(serial).capture();
        let mut history = crate::effect_support::empty_history();
        for step in 1..=12 {
            let next = crate::effect_support::LaterFixture::next(
                &group,
                &history,
                10_000 + serial * 100 + step,
            );
            history = next.evidence.clone();
            group = next.capture();
        }
        items.push(stored(&from_group(&group, &history, serial)));
    }
    let owners: usize = items
        .iter()
        .map(|r| 1 + r.record_history.records.judicial.groups.len())
        .sum();
    assert_eq!(owners, 280);
    let expected = page(crate::participant_support::case_id(), items);
    let query = MeasureAdministrativeReadQuery::new(20, None).unwrap();
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
}
