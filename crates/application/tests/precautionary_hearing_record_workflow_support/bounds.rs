use super::*;
use application::documents::DocumentProcessor;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use std::{io::Read, sync::Arc};

struct NoHash;
impl DocumentHasher for NoHash {
    fn hash_bytes(&self, _: &[u8]) -> Sha256Digest {
        panic!("oversized evidence reached hashing")
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("oversized evidence reached hashing")
    }
}

fn oversized(fixture: Fixture) {
    let (processor, observations): (DocumentProcessor, _) = crate::observed_crypto::processor();
    let validator = Arc::new(Validator::default());
    let service = PrecautionaryHearingRecordService::new(
        Arc::new(fixture.store()),
        Arc::new(identity(fixture.actor)),
        Arc::new(processor),
        validator.clone(),
        Arc::new(NoHash),
        Arc::new(FixedClock(now())),
    );
    assert!(service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
    assert_eq!(validator.calls(), 0);
    assert!(observations.lock().unwrap().events.is_empty());
}

#[test]
fn oversized_history_owners_and_selected_participants_are_refused_before_hashing_or_admission() {
    let prior = Fixture::schedule().operation(at());
    for mutation in 0..4 {
        let mut fixture = Fixture::replace(&prior);
        match mutation {
            0 => {
                fixture.material.history.as_mut().unwrap().captures =
                    vec![prior.capture.clone(); 257]
            }
            1 => {
                let owner = fixture.material.record_history.records.administrative[0].clone();
                fixture.material.record_history.records.administrative = vec![owner; 257];
            }
            2 => {
                let owner = fixture.material.record_history.records.judicial.groups[0].clone();
                fixture.material.record_history.records.judicial.groups = vec![owner; 257];
            }
            _ => {
                let selected = fixture.material.selected_sources.as_mut().unwrap();
                selected.participants = vec![selected.participants[0].clone(); 33];
            }
        }
        oversized(fixture);
    }
}

#[test]
fn replay_refuses_oversized_historical_proof_before_hashing() {
    let fixture = Fixture::schedule();
    let mut original = fixture.operation(at());
    original.history.captures = vec![original.capture.clone(); 257];
    let (processor, observations) = crate::observed_crypto::processor();
    let service = PrecautionaryHearingRecordService::new(
        Arc::new(fixture.replay_store(original)),
        Arc::new(identity(fixture.actor)),
        Arc::new(processor),
        Arc::new(Validator::default()),
        Arc::new(NoHash),
        Arc::new(FixedClock(now())),
    );
    assert!(service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
    assert!(observations.lock().unwrap().events.is_empty());
}
