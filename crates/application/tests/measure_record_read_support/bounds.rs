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
fn counted(store: MockReads) -> (MeasureRecordReadService, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (
        MeasureRecordReadService::new(
            Arc::new(store),
            Arc::new(identity(&reader(Role::Paralegal))),
            Arc::new(CountHasher(count.clone())),
            Arc::new(FixedClock(now())),
        ),
        count,
    )
}

#[test]
fn oversized_page_is_rejected_before_hashing_any_record() {
    let original = initial(10);
    let case_id = original.case_id;
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case_id, vec![original; 21])));
    let (service, count) = counted(store);
    assert!(service
        .list(
            "session",
            case_id,
            MeasureRecordReadQuery::new(20, None).unwrap()
        )
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn owner_and_nested_member_bounds_are_checked_before_hashing_original_proofs() {
    let original = mixed(10).pop().unwrap();
    let actor = reader(Role::Paralegal);
    for kind in READS {
        for mutation in 0..4 {
            let mut returned = original.clone();
            let proof = &mut returned.record_history;
            match mutation {
                0 => {
                    proof.records.judicial.groups =
                        vec![proof.records.judicial.groups[0].clone(); 257]
                }
                1 => {
                    let group = &mut proof.records.judicial.groups[0].capture;
                    group.measures = vec![group.measures[0].clone(); 33];
                }
                2 => {
                    let group = &mut proof.decisions[0].capture;
                    group.measures = vec![group.measures[0].clone(); 33];
                }
                _ => {
                    let a = &mut proof.records.administrative[0].capture;
                    a.records = vec![a.records[0].clone(); 2];
                }
            }
            let (service, count) = counted(successful_store(&actor, &original, returned, kind));
            assert!(read(&service, kind, &original).is_err());
            assert_eq!(count.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn independently_bounded_page_proofs_do_not_acquire_an_aggregate_256_owner_limit() {
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
        items.push(from_group(&group, &history));
    }
    assert_eq!(
        items
            .iter()
            .map(|row| row.record_history.records.judicial.groups.len())
            .sum::<usize>(),
        260
    );
    let expected = page(crate::participant_support::case_id(), items);
    assert_eq!(
        list_result(
            MeasureRecordReadQuery::new(20, None).unwrap(),
            expected.clone()
        )
        .unwrap(),
        expected
    );
}
