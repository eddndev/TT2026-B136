use application::case_stages::*;
use application::cases::{CaseActorSnapshot, CaseInitialStageRegistration};
use application::ApplicationError;
use domain::case_administration::{CaseRevision, InitialCaseStage};
use domain::cases::CaseId;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::{Permission, UserId};
use domain::DomainError;
use std::{io::Read, sync::Mutex};
use time::macros::datetime;

#[test]
fn expectation_keeps_unregistered_separate_from_positive_revisions() {
    assert_eq!(
        CaseStageExpectation::new(0),
        CaseStageExpectation::Unregistered
    );
    assert_eq!(CaseStageExpectation::new(0).get(), 0);
    for value in [1, 7, u32::MAX] {
        assert_eq!(
            CaseStageExpectation::new(value),
            CaseStageExpectation::Revision(CaseStageRevision::new(value).unwrap())
        );
        assert_eq!(CaseStageExpectation::new(value).get(), value);
    }
}

#[test]
fn history_query_has_a_positive_exclusive_cursor_and_bounded_limit() {
    for limit in [1, 100] {
        let query = CaseStageQuery::new(limit, Some(u32::MAX)).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.before_revision().unwrap().get(), u32::MAX);
    }
    assert_eq!(
        CaseStageQuery::new(20, None).unwrap().before_revision(),
        None
    );
    for (limit, before) in [(0, None), (101, None), (u32::MAX, None), (1, Some(0))] {
        assert!(matches!(
            CaseStageQuery::new(limit, before),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn unregistered_has_no_fabricated_revision_or_entry() {
    let current = CurrentCaseStage::Unregistered;
    assert_eq!(current.revision(), None);
    assert_eq!(current.stage(), None);
    assert_eq!(current.entry(), None);
}

#[test]
fn initial_entry_retains_original_administration_provenance() {
    let captured = CaseInitialStageRegistration {
        case_id: CaseId::new(),
        stage_revision: CaseStageRevision::FIRST,
        administration_revision: CaseRevision::FIRST,
        stage: InitialCaseStage::Investigation,
        administration_digest: Sha256Digest::from_array([4; 32]),
        recorded_at: datetime!(2026-09-01 12:00 UTC),
        recorded_by: CaseActorSnapshot {
            id: UserId::new(),
            email: "historical@example.com".into(),
        },
    };
    let entry = CaseStageEntry::Initial(captured.clone());
    assert_eq!(entry.case_id(), captured.case_id);
    assert_eq!(entry.stage_revision(), CaseStageRevision::FIRST);
    assert_eq!(entry.stage(), CaseStage::Investigation);
    assert_eq!(entry.recorded_at(), captured.recorded_at);
    assert_eq!(entry.recorded_by(), &captured.recorded_by);
    let current = CurrentCaseStage::Registered(Box::new(entry.clone()));
    assert_eq!(current.entry(), Some(&entry));
    assert_eq!(current.revision(), Some(CaseStageRevision::FIRST));
    assert_eq!(current.stage(), Some(CaseStage::Investigation));
    assert_eq!(entry, CaseStageEntry::Initial(captured));
}

#[test]
fn each_audit_action_has_its_own_name_and_permission() {
    for (action, name, permission) in [
        (
            CaseStageAction::Read,
            "case.stage_read",
            Permission::ReadCaseStage,
        ),
        (
            CaseStageAction::History,
            "case.stage_history_read",
            Permission::ReadCaseStage,
        ),
        (
            CaseStageAction::Adopt,
            "case.stage_adopted",
            Permission::ManageCaseStage,
        ),
        (
            CaseStageAction::Transition,
            "case.stage_transitioned",
            Permission::ManageCaseStage,
        ),
    ] {
        assert_eq!(action.as_str(), name);
        assert_eq!(action.permission(), permission);
    }
}

struct CaptureHasher(Mutex<Vec<u8>>);
impl DocumentHasher for CaptureHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        *self.0.lock().unwrap() = bytes.to_vec();
        Sha256Digest::from_array([42; 32])
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("canonical values use the byte hashing port")
    }
}

#[test]
fn stage_digest_delegates_exact_canonical_values_to_the_hash_port() {
    let hasher = CaptureHasher(Mutex::new(Vec::new()));
    let change = CaseStageChange::Adopt(StageAdoption::new(
        CaseStage::Intermediate,
        DeclaredStageTime::instant(datetime!(2026-09-01 12:00 UTC)).unwrap(),
        StageNote::new("Previously known").unwrap(),
        StageSupportRef::new(
            domain::crypto::DocumentVersionRef {
                id: domain::crypto::DocumentId::new(),
                version: domain::crypto::DocumentVersion::initial(),
            },
            Sha256Digest::from_array([1; 32]),
        ),
    ));
    assert_eq!(
        case_stage_digest(&hasher, &change),
        Sha256Digest::from_array([42; 32])
    );
    assert_eq!(*hasher.0.lock().unwrap(), change.canonical_bytes());
}
