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
fn counted(store: MockReads) -> (MeasureDecisionRecordReadService, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (
        MeasureDecisionRecordReadService::new(
            Arc::new(store),
            Arc::new(identity(&reader(Role::Paralegal))),
            Arc::new(CountHasher(count.clone())),
            Arc::new(FixedClock(now())),
        ),
        count,
    )
}

#[test]
fn mixed_page_and_nested_original_proof_limits_precede_any_hashing() {
    let original = v2(10);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        for mutation in 0..4 {
            let mut returned = original.clone();
            let MeasureDecisionRecordReceipt::V2(value) = &mut returned else {
                unreachable!()
            };
            match mutation {
                0 => {
                    value.record_history.records.judicial.groups =
                        vec![value.record_history.records.judicial.groups[0].clone(); 257]
                }
                1 => {
                    let group = &mut value.record_history.records.judicial.groups[0].capture;
                    group.measures = vec![group.measures[0].clone(); 33];
                }
                2 => {
                    let capture = &mut value.record_history.records.administrative[0].capture;
                    capture.records = vec![capture.records[0].clone(); 2];
                }
                _ => value.group.measures = vec![value.group.measures[0].clone(); 33],
            }
            let (service, count) = counted(successful_store(&actor, &original, returned, kind));
            assert!(read(&service, kind, &original).is_err());
            assert_eq!(count.load(Ordering::SeqCst), 0);
        }
    }
    let case = original.case_id();
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
            MeasureDecisionReadQuery::new(20, None).unwrap()
        )
        .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn independently_bounded_mixed_pages_have_no_new_aggregate_owner_limit() {
    let mut items = Vec::new();
    for serial in 1..=20 {
        let original = root_fixture(serial).capture();
        let mut fixture = FixtureV2::from_v1(&original, &crate::effect_support::empty_history());
        fixture.identities(100_000 + serial * 100);
        for step in 1..12 {
            fixture = FixtureV2::next(
                &fixture.capture(),
                &fixture.history,
                100_000 + serial * 100 + step,
            );
        }
        items.push(from_v2(&fixture));
    }
    items.sort_by_key(|row| row.origin().decision_id.as_uuid());
    let total: usize = items
        .iter()
        .map(|row| {
            let MeasureDecisionRecordReceipt::V2(value) = row else {
                unreachable!()
            };
            value.record_history.records.judicial.groups.len()
                + value.record_history.decisions.len()
                + 1
        })
        .sum();
    assert_eq!(total, 260);
    let expected = page(crate::participant_support::case_id(), items);
    assert_eq!(
        list_result(
            MeasureDecisionReadQuery::new(20, None).unwrap(),
            expected.clone()
        )
        .unwrap(),
        expected
    );
}
