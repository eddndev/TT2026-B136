mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_readback_support;
mod procedural_fact_backend_support;

use application::{
    cases::{CaseEditableValues, CaseRepository, CaseRevisionExpectation},
    deadline_reevaluation::ObservationRole,
    deadline_tracking::TrackingPolicy,
    deadlines::*,
};
use deadline_backend_support::*;
use deadline_tracked_backend_support::*;
use deadline_tracked_readback_support::*;
use domain::{cases::CaseMetadata, crypto::DocumentHasher};
use infrastructure::RingSha256Hasher;
use time::Duration;

#[test]
fn coherent_tracking_hashes_cannot_authenticate_fabricated_profile_evidence() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let command = setup(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    let original = repository.commit(db.owner, prepared).unwrap();
    assert_readback(&db, repository.as_ref(), std::slice::from_ref(&original));
    let mut forged = original.clone();
    let profile = forged
        .tracking
        .as_mut()
        .unwrap()
        .observations
        .entries
        .iter_mut()
        .find(|entry| entry.role == ObservationRole::Profile)
        .unwrap();
    let authentic = profile.evidence_digest;
    profile.evidence_digest = RingSha256Hasher.hash_bytes(b"fabricated profile evidence");
    assert_ne!(profile.evidence_digest, authentic);
    write_coherent_tracking(&mut db, &mut forged);
    assert_eq!(forged.definition, original.definition);
    assert_eq!(forged.calculation, original.calculation);
    assert_eq!(forged.recorded_by, original.recorded_by);
    assert_ne!(forged.receipt, original.receipt);
    assert_rejected_by_reads_and_inventory(&mut db, repository.as_ref(), &forged);
}

#[test]
fn coherent_fixed_observation_cannot_invent_a_resolution_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let command = setup(&db);
    let mut selected_policies = policies();
    selected_policies.source = TrackingPolicy::Fixed;
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(selected_policies));
    let original = repository.commit(db.owner, prepared).unwrap();
    assert_readback(&db, repository.as_ref(), std::slice::from_ref(&original));
    let mut forged = original.clone();
    let source = forged
        .tracking
        .as_mut()
        .unwrap()
        .observations
        .entries
        .iter_mut()
        .find(|entry| entry.role == ObservationRole::Source)
        .unwrap();
    assert_eq!(source.revision, 1);
    // The fixture persists only revision one. Fixed permits a later observed
    // revision structurally, but that revision must still exist in history.
    source.revision = 2;
    write_coherent_tracking(&mut db, &mut forged);
    assert_eq!(forged.definition, original.definition);
    assert_eq!(forged.calculation, original.calculation);
    assert_eq!(forged.review_state(), original.review_state());
    assert_rejected_by_reads_and_inventory(&mut db, repository.as_ref(), &forged);
}

#[test]
fn later_administration_does_not_replace_exact_v2_tracking_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let captured = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Unrevised,
            CaseEditableValues::new(
                CaseMetadata::new("Captured administration", "REF-CAPTURED").unwrap(),
                None,
            ),
            db.at,
        )
        .unwrap()
        .administration;
    let command = setup(&db);
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    let first = repository.commit(db.owner, prepared).unwrap();
    assert_eq!(first.tracking.as_ref().unwrap().administration, captured);
    let first_row = revision_row(&mut db, &first);
    let first_bytes = stored_evidence(&mut db, &first);
    let pending = tracked_prepared(&db, repository.as_ref(), &attention(&first), None);
    let expected = prepared_detail(&db, &pending);
    let later = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseEditableValues::new(
                CaseMetadata::new("Later administration", "REF-LATER").unwrap(),
                None,
            ),
            db.at + Duration::hours(1),
        )
        .unwrap()
        .administration;
    assert_eq!(captured.revision().unwrap().get(), 1);
    assert_eq!(later.revision().unwrap().get(), 2);
    assert_ne!(captured.values(), later.values());
    assert_ne!(
        captured.snapshot().unwrap().changed_at,
        later.snapshot().unwrap().changed_at
    );
    let second = repository.commit(db.owner, pending).unwrap();
    assert_eq!(second, expected);
    assert_eq!(second.calculation, first.calculation);
    assert_eq!(second.tracking, first.tracking);
    let actual = &second.tracking.as_ref().unwrap().administration;
    assert_eq!(actual, &captured);
    assert_eq!(
        actual.snapshot().unwrap().changed_at.offset(),
        captured.snapshot().unwrap().changed_at.offset()
    );
    assert_eq!(
        actual.snapshot().unwrap().changed_at.unix_timestamp_nanos(),
        captured
            .snapshot()
            .unwrap()
            .changed_at
            .unix_timestamp_nanos()
    );
    assert_eq!(revision_row(&mut db, &first), first_row);
    assert_eq!(stored_evidence(&mut db, &first), first_bytes);
    let before = snapshot(&mut db);
    drop(repository);
    let reopened = store(&db);
    assert_eq!(snapshot(&mut db), before);
    assert_readback(&db, reopened.as_ref(), &[first, second]);
}
