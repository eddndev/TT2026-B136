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
fn mixed_resolver_checks_combined_owner_row_and_selection_limits_before_hashing() {
    let (fixture, capture, original) = chain();
    let selected = record_reference(&capture.records[0]);
    for mutation in 0..4 {
        let mut evidence = original.clone();
        let mut selections = vec![selected];
        match mutation {
            0 => evidence.administrative = vec![evidence.administrative[0].clone(); 256],
            1 => {
                let group = &mut evidence.judicial.groups[0].capture;
                group.measures = vec![group.measures[0].clone(); 8192];
            }
            2 => {
                let administrative = &mut evidence.administrative[0].capture;
                administrative.records = vec![administrative.records[0].clone(); 33];
            }
            _ => selections = vec![selected; 33],
        }
        let hasher = CountHasher::default();
        assert!(resolve_measure_records(&hasher, fixture.case_id, &selections, &evidence).is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn repeated_preparation_counts_candidate_before_hashing_ancestor_owners_and_rows() {
    let (fixture, capture, _) = chain();
    for owners in [true, false] {
        let mut next = RecordFixture::next(&capture, &fixture.history, 2);
        if owners {
            next.history.administrative = vec![next.history.administrative[0].clone(); 255];
        } else {
            let group = &mut next.history.judicial.groups[0].capture;
            group.measures = vec![group.measures[0].clone(); 8190];
        }
        let hasher = CountHasher::default();
        assert!(prepare_measure_record_correction_with_history(
            &hasher,
            &next.actor,
            next.case_id,
            next.command,
            next.context,
            &next.history,
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn mixed_capture_validation_preflights_oversized_administrative_ancestors() {
    let (mut fixture, capture, _) = chain();
    let prior = &mut fixture.history.administrative[0].capture;
    prior.records = vec![prior.records[0].clone(); 33];
    let hasher = CountHasher::default();
    assert!(measure_administrative_capture_with_history_matches(
        &hasher,
        &capture,
        &fixture.history
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}
