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
fn correction_owner_and_row_budgets_are_checked_before_hashing_history() {
    for owners in [true, false] {
        let mut fixture = CorrectionFixture::initial();
        if owners {
            fixture.history.groups = vec![fixture.history.groups[0].clone(); 256];
        } else {
            let group = &mut fixture.history.groups[0].capture;
            group.measures = vec![group.measures[0].clone(); 8192];
        }
        let hasher = CountHasher::default();
        assert!(prepare_measure_record_correction(
            &hasher,
            &fixture.actor,
            fixture.case_id,
            fixture.command,
            fixture.context,
            &fixture.history,
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn reconstruction_rejects_oversized_administrative_records_before_hashing() {
    let fixture = CorrectionFixture::initial();
    let mut capture = fixture.capture();
    capture.records = vec![capture.records[0].clone(); 33];
    let hasher = CountHasher::default();
    assert!(measure_administrative_capture_matches(&hasher, &capture, &fixture.history).is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}
