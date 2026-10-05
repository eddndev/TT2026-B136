use super::*;
use domain::DomainError;
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
fn decision_history_preparation_rejects_oversized_g2_owners_and_participants_before_hashing() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let original = DecisionReviewFixture::schedule(
        vec![reference_v2(&group.measures[0])],
        append_v2(&judicial.history, &group),
    );
    for mutation in 0..3 {
        let mut fixture = original.clone();
        match mutation {
            0 => {
                fixture.decision_history.decisions =
                    vec![fixture.decision_history.decisions[0].clone(); 256]
            }
            1 => {
                let owner = &mut fixture.decision_history.decisions[0].capture;
                owner.measures = vec![owner.measures[0].clone(); 33];
            }
            _ => {
                fixture.hearing.sources.participants =
                    vec![fixture.hearing.sources.participants[0].clone(); 33]
            }
        }
        let hasher = CountHasher::default();
        let h = fixture.hearing;
        assert!(prepare_precautionary_hearing_with_decision_history(
            &hasher,
            &h.actor,
            h.case_id,
            h.command,
            PrecautionaryHearingDecisionPreparationMaterial {
                observed_context: h.context,
                sources: h.sources,
                predecessor: None,
                decision_history: &fixture.decision_history,
            },
        )
        .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn decision_history_prefix_and_nested_projection_limits_precede_hashing() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let evidence = append_v2(&judicial.history, &group);
    let fixture =
        DecisionReviewFixture::schedule(vec![reference_v2(&group.measures[0])], evidence.clone());
    let capture = fixture.capture(None, group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &capture, &evidence).unwrap();
    let hasher = CountHasher::default();
    assert!(precautionary_hearing_history_with_decision_history_matches(
        &hasher,
        &vec![capture.clone(); 257],
        &origin,
        &evidence
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    let mut oversized = capture;
    oversized.review.participants = vec![oversized.review.participants[0].clone(); 33];
    assert!(precautionary_hearing_receipt_with_decision_history_matches(
        &hasher, &oversized, &evidence
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    let cancellation = DecisionReviewFixture::cancel(&oversized, evidence.clone());
    let h = cancellation.hearing;
    assert!(prepare_precautionary_hearing_with_decision_history(
        &hasher,
        &h.actor,
        h.case_id,
        h.command,
        PrecautionaryHearingDecisionPreparationMaterial {
            observed_context: h.context,
            sources: h.sources,
            predecessor: Some(&oversized),
            decision_history: &evidence,
        },
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}
