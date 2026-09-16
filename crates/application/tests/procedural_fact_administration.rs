use application::{
    cases::{CaseActorSnapshot, CaseAdministrationSnapshot, CurrentCaseAdministration},
    procedural_facts::{validate_fact_administration, ProceduralFactError},
    ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    DomainError,
};
use std::{cell::RefCell, io::Read};
use uuid::Uuid;

#[derive(Default)]
struct RecordingHasher {
    seen: RefCell<Vec<Vec<u8>>>,
}
impl DocumentHasher for RecordingHasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        self.seen.borrow_mut().push(data.to_vec());
        digest()
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("administration values must use the byte-slice hashing port")
    }
}
fn digest() -> Sha256Digest {
    Sha256Digest::from_array([7; 32])
}
fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
fn metadata(title: &str, reference: &str) -> CaseMetadata {
    CaseMetadata::new(title, reference).unwrap()
}
fn baseline() -> CurrentCaseAdministration {
    CurrentCaseAdministration::Unrevised(metadata("Basic case", "REF-1"))
}
fn recorded(revision: u32) -> CurrentCaseAdministration {
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case_id(),
        revision: CaseRevision::new(revision).unwrap(),
        values: CaseAdministrationValues::basic(metadata("Basic case", "REF-1")),
        values_digest: digest(),
        changed_at: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        changed_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(2)),
            email: "actor@example.com".into(),
        },
    }))
}
fn snapshot_mut(value: &mut CurrentCaseAdministration) -> &mut CaseAdministrationSnapshot {
    let CurrentCaseAdministration::Recorded(snapshot) = value else {
        panic!("recorded administration fixture expected")
    };
    snapshot
}
fn assert_inconsistent(result: Result<(), ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn unrevised_active_baseline_needs_no_digest_revision_or_complete_profile() {
    let hasher = RecordingHasher::default();
    let observed = baseline();
    let original = observed.clone();
    assert!(validate_fact_administration(&hasher, case_id(), &observed, None).is_ok());
    assert_eq!(observed, original);
    assert_eq!(observed.revision(), None);
    assert!(observed.values().profile().is_none());
    assert!(hasher.seen.borrow().is_empty());
}

#[test]
fn recorded_active_values_are_hashed_through_the_existing_canonical_bytes() {
    let hasher = RecordingHasher::default();
    let observed = recorded(1);
    assert!(validate_fact_administration(&hasher, case_id(), &observed, None).is_ok());
    assert_eq!(
        *hasher.seen.borrow(),
        vec![observed.values().canonical_bytes()]
    );
    assert!(observed.values().profile().is_none());
}

#[test]
fn forged_observed_digest_is_a_storage_inconsistency() {
    let mut observed = recorded(1);
    snapshot_mut(&mut observed).values_digest = Sha256Digest::from_array([8; 32]);
    assert_inconsistent(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &observed,
        None,
    ));
}

#[test]
fn observed_recorded_snapshot_must_belong_to_the_requested_case() {
    let mut observed = recorded(1);
    snapshot_mut(&mut observed).case_id = CaseId::from_uuid(Uuid::from_u128(9));
    assert_inconsistent(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &observed,
        None,
    ));
}

#[test]
fn observed_closure_rejects_a_change_with_or_without_captured_history() {
    let mut observed = recorded(2);
    let snapshot = snapshot_mut(&mut observed);
    snapshot.values = snapshot
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    let captured = recorded(1);
    for prior in [None, Some(&captured)] {
        assert!(matches!(
            validate_fact_administration(&RecordingHasher::default(), case_id(), &observed, prior),
            Err(ApplicationError::CaseClosed)
        ));
    }
}

#[test]
fn matching_unrevised_metadata_is_preserved_without_hashing_or_new_revision() {
    let hasher = RecordingHasher::default();
    let observed = baseline();
    assert!(validate_fact_administration(&hasher, case_id(), &observed, Some(&observed)).is_ok());
    assert!(hasher.seen.borrow().is_empty());
    assert!(observed.snapshot().is_none());
}

#[test]
fn an_unrevised_baseline_cannot_silently_change_either_metadata_field() {
    let captured = baseline();
    for metadata in [
        metadata("Other title", "REF-1"),
        metadata("Basic case", "REF-2"),
    ] {
        let observed = CurrentCaseAdministration::Unrevised(metadata);
        assert_inconsistent(validate_fact_administration(
            &RecordingHasher::default(),
            case_id(),
            &observed,
            Some(&captured),
        ));
    }
}

#[test]
fn an_unrevised_capture_may_advance_to_recorded_administration() {
    let captured = baseline();
    let mut observed = recorded(4);
    snapshot_mut(&mut observed).values =
        CaseAdministrationValues::basic(metadata("Updated", "REF-2"));
    assert!(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &observed,
        Some(&captured),
    )
    .is_ok());
}

#[test]
fn recorded_capture_cannot_fall_back_to_an_unrevised_baseline() {
    assert_inconsistent(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &baseline(),
        Some(&recorded(1)),
    ));
}

#[test]
fn recorded_capture_must_have_valid_scope_and_digest() {
    for corrupt_scope in [false, true] {
        let mut captured = recorded(1);
        if corrupt_scope {
            snapshot_mut(&mut captured).case_id = CaseId::from_uuid(Uuid::from_u128(9));
        } else {
            snapshot_mut(&mut captured).values_digest = Sha256Digest::from_array([8; 32]);
        }
        assert_inconsistent(validate_fact_administration(
            &RecordingHasher::default(),
            case_id(),
            &recorded(2),
            Some(&captured),
        ));
    }
}

#[test]
fn observed_revision_cannot_precede_a_valid_recorded_capture() {
    assert_inconsistent(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &recorded(2),
        Some(&recorded(3)),
    ));
}

#[test]
fn matching_recorded_snapshots_are_valid_including_the_maximum_revision() {
    for revision in [1, u32::MAX] {
        let observed = recorded(revision);
        assert!(validate_fact_administration(
            &RecordingHasher::default(),
            case_id(),
            &observed,
            Some(&observed),
        )
        .is_ok());
    }
}

#[test]
fn an_equal_revision_requires_the_complete_immutable_snapshot() {
    let captured = recorded(3);
    for field in ["actor_id", "actor_email", "changed_at", "values"] {
        let mut observed = captured.clone();
        let snapshot = snapshot_mut(&mut observed);
        match field {
            "actor_id" => snapshot.changed_by.id = UserId::from_uuid(Uuid::from_u128(9)),
            "actor_email" => snapshot.changed_by.email = "other@example.com".into(),
            "changed_at" => snapshot.changed_at += time::Duration::seconds(1),
            "values" => {
                snapshot.values = CaseAdministrationValues::basic(metadata("Other", "REF-1"))
            }
            _ => unreachable!(),
        }
        assert_inconsistent(validate_fact_administration(
            &RecordingHasher::default(),
            case_id(),
            &observed,
            Some(&captured),
        ));
    }
}

#[test]
fn newer_administration_is_allowed_and_both_value_canons_are_checked() {
    let hasher = RecordingHasher::default();
    let captured = recorded(1);
    let mut observed = recorded(9);
    let snapshot = snapshot_mut(&mut observed);
    snapshot.values = CaseAdministrationValues::basic(metadata("Updated title", "REF-2"));
    snapshot.changed_by.email = "new-actor@example.com".into();
    snapshot.changed_at += time::Duration::days(1);
    assert!(validate_fact_administration(&hasher, case_id(), &observed, Some(&captured)).is_ok());
    let seen = hasher.seen.borrow();
    assert_eq!(seen.len(), 2);
    assert!(seen.contains(&observed.values().canonical_bytes()));
    assert!(seen.contains(&captured.values().canonical_bytes()));
}

#[test]
fn a_closed_historical_capture_does_not_override_current_active_status() {
    let mut captured = recorded(2);
    let snapshot = snapshot_mut(&mut captured);
    snapshot.values = snapshot
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    assert!(validate_fact_administration(
        &RecordingHasher::default(),
        case_id(),
        &recorded(3),
        Some(&captured),
    )
    .is_ok());
}
