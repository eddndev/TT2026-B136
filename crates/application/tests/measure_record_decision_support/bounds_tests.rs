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
fn v2_preparation_checks_combined_owner_and_nested_shape_limits_before_hashing() {
    let (prior_fixture, prior) = corrected();
    let original = FixtureV2::confirm(&prior, &prior_fixture.history);
    let group = original.capture();
    let entry = crate::record_decision_support::append_v2(&original.history, &group)
        .decisions
        .remove(0);
    for mutation in 0..4 {
        let mut fixture = original.clone();
        match mutation {
            0 => fixture.history.decisions = vec![entry.clone(); 254],
            1 => fixture.material.predecessors = vec![fixture.material.predecessors[0].clone(); 33],
            2 => {
                fixture.material.result_sources =
                    vec![fixture.material.result_sources[0].clone(); 33]
            }
            _ => {
                let mut oversized = entry.clone();
                oversized.capture.measures = vec![oversized.capture.measures[0].clone(); 8190];
                fixture.history.decisions.push(oversized);
            }
        }
        let hasher = CountHasher::default();
        assert!(prepare_measure_decision_with_record_history(
            &hasher,
            &fixture.actor,
            fixture.case_id,
            fixture.command,
            fixture.material,
            &fixture.history,
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn unified_resolver_and_v2_matcher_preflight_oversized_new_family_rows() {
    let (prior_fixture, prior) = corrected();
    let fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let group = fixture.capture();
    let selected = crate::record_decision_support::reference_v2(&group.measures[0]);
    let mut oversized = group.clone();
    oversized.measures = vec![oversized.measures[0].clone(); 33];
    let hasher = CountHasher::default();
    assert!(measure_decision_group_v2_matches(&hasher, &oversized, &fixture.history).is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    let mut evidence = crate::record_decision_support::append_v2(&fixture.history, &group);
    evidence.decisions[0].capture = oversized;
    assert!(resolve_measure_records_with_decision_history(
        &hasher,
        fixture.case_id,
        &[selected],
        &evidence
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}
