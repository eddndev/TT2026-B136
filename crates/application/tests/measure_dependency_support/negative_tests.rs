use crate::measure_dependency_support::*;
use domain::cases::CaseId;
use domain::crypto::{DocumentHasher, Sha256Digest};
use uuid::Uuid;

#[path = "bounds_negative.rs"]
mod bounds;
#[path = "cycle_negative.rs"]
mod cycles;
#[path = "forest_negative.rs"]
mod forest;
#[path = "prefix_negative.rs"]
mod prefixes;

fn reject(target: PrecautionaryMeasureRef, inventory: &MeasureAdministrativeDependencyInventory) {
    assert!(inspect_measure_administrative_dependencies(
        &Hasher,
        CaseId::from_uuid(Uuid::from_u128(1)),
        target,
        inventory,
    )
    .is_err());
}

fn refresh_hearing(capture: &mut PrecautionaryHearingCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

fn reviewed() -> (
    PrecautionaryMeasureRef,
    MeasureAdministrativeDependencyInventory,
) {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let mut inventory = judicial_inventory(&base);
    let hearing = DecisionReviewFixture::schedule(vec![selected], inventory.records.clone())
        .capture(None, base.recorded_at);
    inventory
        .hearings
        .push(hearing_prefix(vec![hearing], &inventory.records));
    (selected, inventory)
}
