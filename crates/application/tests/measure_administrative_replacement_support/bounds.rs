use crate::replacement_support::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    DomainError,
};
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
    fn hash_stream(&self, source: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(source)
    }
}

#[test]
fn combined_owner_overflow_and_extra_joint_member_reject_before_hashing() {
    let fixture = ReplacementFixture::initial();
    let capture = fixture.capture();
    for mutation in 0..2 {
        let mut history = fixture.history.clone();
        if mutation == 0 {
            history.records.judicial.groups = vec![history.records.judicial.groups[0].clone(); 256];
        } else {
            history = append(&fixture, &capture);
            let owner = &mut history.records.administrative[0].capture;
            owner.records.push(owner.records[0].clone());
        }
        let hasher = CountHasher::default();
        assert!(
            prepare_measure_administrative_replacement_with_decision_history(
                &hasher,
                &fixture.actor,
                fixture.case_id,
                fixture.command.clone(),
                MeasureAdministrativeReplacementMaterial {
                    context: fixture.context.clone(),
                    subject: fixture.subject.clone()
                },
                &history,
            )
            .is_err()
        );
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
    }
}
