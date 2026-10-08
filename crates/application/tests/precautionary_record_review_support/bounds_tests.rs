use super::*;
use domain::{crypto::Sha256Digest, DomainError};
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
fn mixed_hearing_history_rejects_more_than_256_captures_before_hashing() {
    let (fixture, prior) = corrected();
    let (hearing, evidence) = review_fixture(&prior, &fixture.history);
    let capture = prepare(&hearing, &evidence, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &capture, &evidence).unwrap();
    let captures = vec![capture; 257];
    let hasher = CountHasher::default();
    assert!(precautionary_hearing_history_with_record_history_matches(
        &hasher, &captures, &origin, &evidence
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}

#[test]
fn mixed_hearing_preparation_rejects_oversized_sources_and_predecessor_projections_before_hashing()
{
    let (fixture, prior) = corrected();
    let (original, evidence) = review_fixture(&prior, &fixture.history);
    let capture = prepare(&original, &evidence, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    for oversized_predecessor in [false, true] {
        let mut hearing = if oversized_predecessor {
            HearingFixture::cancel(&capture)
        } else {
            original.clone()
        };
        let mut predecessor = capture.clone();
        if oversized_predecessor {
            predecessor.review.participants = vec![predecessor.review.participants[0].clone(); 33];
        } else {
            hearing.sources.participants = vec![hearing.sources.participants[0].clone(); 33];
        }
        let hasher = CountHasher::default();
        assert!(prepare_precautionary_hearing_with_record_history(
            &hasher,
            &hearing.actor,
            hearing.case_id,
            hearing.command,
            PrecautionaryHearingRecordPreparationMaterial {
                observed_context: hearing.context,
                sources: hearing.sources,
                predecessor: oversized_predecessor.then_some(&predecessor),
                record_history: &evidence,
            },
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn mixed_hearing_receipt_preflights_each_participant_array_before_hashing() {
    let (fixture, prior) = corrected();
    let (hearing, evidence) = review_fixture(&prior, &fixture.history);
    let original = prepare(&hearing, &evidence, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    for projection in [false, true] {
        let mut capture = original.clone();
        if projection {
            capture.review.participants = vec![capture.review.participants[0].clone(); 33];
        } else {
            capture.review.sources.participants =
                vec![capture.review.sources.participants[0].clone(); 33];
        }
        let hasher = CountHasher::default();
        assert!(precautionary_hearing_receipt_with_record_history_matches(
            &hasher, &capture, &evidence
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}
