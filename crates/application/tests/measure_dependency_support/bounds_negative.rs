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
    fn hash_stream(&self, source: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(source)
    }
}

fn bounded_rejection(
    target: PrecautionaryMeasureRef,
    inventory: &MeasureAdministrativeDependencyInventory,
) {
    let hasher = CountHasher::default();
    assert!(inspect_measure_administrative_dependencies(
        &hasher,
        CaseId::from_uuid(Uuid::from_u128(1)),
        target,
        inventory,
    )
    .is_err());
    assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
}

#[test]
fn aggregate_owner_prefix_and_historical_capture_limits_precede_hashing() {
    let (selected, original) = reviewed();
    for mutation in 0..4 {
        let mut inventory = original.clone();
        match mutation {
            0 => {
                inventory.records.records.judicial.groups =
                    vec![inventory.records.records.judicial.groups[0].clone(); 257]
            }
            1 => inventory.hearings = vec![inventory.hearings[0].clone(); 257],
            2 => {
                inventory.hearings[0].captures =
                    vec![inventory.hearings[0].captures[0].clone(); 257]
            }
            _ => {
                inventory.hearings = vec![inventory.hearings[0].clone(); 2];
                for prefix in &mut inventory.hearings {
                    prefix.captures = vec![prefix.captures[0].clone(); 129];
                }
            }
        }
        bounded_rejection(selected, &inventory);
    }
}

#[test]
fn nested_participant_projection_and_owner_row_limits_precede_hashing() {
    let (selected, original) = reviewed();
    for mutation in 0..4 {
        let mut inventory = original.clone();
        match mutation {
            0 => {
                let r = &mut inventory.hearings[0].captures[0].review;
                r.sources.participants = vec![r.sources.participants[0].clone(); 33];
            }
            1 => {
                let r = &mut inventory.hearings[0].captures[0].review;
                r.participants = vec![r.participants[0].clone(); 33];
            }
            2 => {
                let g = &mut inventory.records.records.judicial.groups[0].capture;
                g.measures = vec![g.measures[0].clone(); 33];
            }
            _ => {
                let anchor = &inventory.hearings[0].captures[0];
                let group = anchor_v2(anchor, &inventory.records, 1);
                inventory.records = append_v2(&inventory.records, &group);
                let Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) =
                    &mut inventory.records.decisions[0].capture.decision.anchor
                else {
                    panic!("anchor required")
                };
                capture.review.participants = vec![capture.review.participants[0].clone(); 33];
            }
        }
        bounded_rejection(selected, &inventory);
    }
}
