use super::*;
use domain::{crypto::DocumentHasher, DomainError};
use std::{
    io::Read,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct CountHasher(AtomicUsize);
impl DocumentHasher for CountHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_bytes(bytes)
    }
    fn hash_stream(&self, input: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(input)
    }
}

#[test]
fn oversized_prefix_owners_and_nested_arrays_reject_before_hashing() {
    let saved = operation(40);
    let actor = reader(Role::Owner);
    for mutation in 0..7 {
        let mut changed = saved.clone();
        match mutation {
            0 => changed.history.captures = vec![changed.capture.clone(); 257],
            1 => {
                changed.history.record_history.decisions =
                    vec![changed.history.record_history.decisions[0].clone(); 256]
            }
            2 => {
                let group = &mut changed.history.record_history.decisions[0].capture;
                group.measures = vec![group.measures[0].clone(); 33];
            }
            3 => {
                let group = &mut changed.history.record_history.records.judicial.groups[0].capture;
                group.measures = vec![group.measures[0].clone(); 33];
            }
            4 => {
                let group = &mut changed.history.record_history.records.administrative[0].capture;
                group.records = vec![group.records[0].clone(); 2];
            }
            5 => {
                changed.capture.review.sources.participants =
                    vec![changed.capture.review.sources.participants[0].clone(); 33];
                changed.history.captures[0] = changed.capture.clone();
            }
            _ => {
                changed.capture.review.participants =
                    vec![changed.capture.review.participants[0].clone(); 33];
                changed.history.captures[0] = changed.capture.clone();
            }
        }
        let store = successful_store(&actor, &saved, changed, ReadKind::Exact);
        let hasher = Arc::new(CountHasher::default());
        let service = PrecautionaryHearingRecordReadService::new(
            Arc::new(store),
            Arc::new(identity(&actor)),
            hasher.clone(),
            clock(),
        );
        assert!(read(&service, &saved, ReadKind::Exact).is_err());
        assert_eq!(
            hasher.0.load(Ordering::SeqCst),
            0,
            "hashed oversized mutation {mutation}"
        );
    }
}

#[test]
fn oversized_page_rejects_before_hashing_any_original_operation() {
    let saved = operation(40);
    let case = saved.capture.review.case_id;
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case, vec![saved; 21])));
    let hasher = Arc::new(CountHasher::default());
    let service = PrecautionaryHearingRecordReadService::new(
        Arc::new(store),
        Arc::new(identity(&reader(Role::Owner))),
        hasher.clone(),
        clock(),
    );
    assert!(service
        .list(
            "session",
            case,
            PrecautionaryHearingReadQuery::new(20, None).unwrap()
        )
        .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}

#[test]
fn twenty_individually_bounded_histories_may_have_more_than_256_owners_in_union() {
    use domain::precautionary_measures::{
        MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureEffect, MeasureProposal,
    };
    let mut items = Vec::new();
    for serial in 0..20 {
        let mut fixture = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::single());
        fixture.identities(10_000 + serial * 100);
        let id = MeasureId::from_uuid(uuid::Uuid::from_u128(50_000 + serial));
        let MeasureEffect::Impose(proposal) = fixture.command.outcome.changes().unwrap()[0].clone()
        else {
            unreachable!()
        };
        fixture.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                MeasureEffect::Impose(MeasureProposal {
                    id,
                    values: proposal.values,
                }),
            ]))
            .unwrap();
        fixture.material.result_sources[0].id = id;
        let mut group = fixture.capture();
        for step in 1..13 {
            fixture = FixtureV2::next(&group, &fixture.history, 10_000 + serial * 100 + step);
            group = fixture.capture();
        }
        items.push(scheduled(
            100 + serial,
            vec![reference_v2(&group.measures[0])],
            append_v2(&fixture.history, &group),
            group.recorded_at,
        ));
    }
    assert_eq!(
        items
            .iter()
            .map(|i| i.history.record_history.decisions.len())
            .sum::<usize>(),
        260
    );
    let case = items[0].capture.review.case_id;
    let expected = items.clone();
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case, items)));
    assert_eq!(
        service(store, identity(&reader(Role::Paralegal)), clock())
            .list(
                "session",
                case,
                PrecautionaryHearingReadQuery::new(20, None).unwrap()
            )
            .unwrap()
            .items,
        expected
    );
}
