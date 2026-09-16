use application::credential_trust::{CredentialTrustExpectation, CredentialTrustRevision};
use application::typed_participants::*;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};

fn subject() -> SubjectRevisionRef {
    SubjectRevisionRef {
        id: CaseSubjectId::new(),
        revision: SubjectRevision::initial(),
        values_digest: Sha256Digest::from_bytes(&[0; 32]).unwrap(),
    }
}
fn values(subject: SubjectRevisionRef) -> TypedParticipantValues {
    let evidence = ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: DocumentId::new(),
            version: DocumentVersion::new(1).unwrap(),
        },
        Sha256Digest::from_bytes(&[1; 32]).unwrap(),
        "page 1",
    )
    .unwrap();
    let role = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::ControlJudge(ControlJudgeProfile::new("Court").unwrap()),
        evidence,
    )
    .unwrap();
    TypedParticipantValues::new(subject, DirectoryStatus::Active, role)
}
#[test]
fn proposal_rejects_subject_incoherence_and_exhausted_revision() {
    let reference = subject();
    assert!(ParticipantProposal::new(
        SubjectChange::Keep(subject()),
        ParticipantId::new(),
        ParticipantExpectation::Absent,
        values(reference)
    )
    .is_err());
    assert!(ParticipantProposal::new(
        SubjectChange::Keep(reference),
        ParticipantId::new(),
        ParticipantExpectation::Revision(ParticipantRevision::new(u32::MAX).unwrap()),
        values(reference)
    )
    .is_err());
    assert!(ParticipantProposal::new(
        SubjectChange::Keep(reference),
        ParticipantId::new(),
        ParticipantExpectation::Absent,
        values(reference)
    )
    .is_ok());
}
#[test]
fn subject_queries_are_bounded_and_reject_original_controls() {
    assert!(SubjectQuery::new(0, None, None, None).is_err());
    assert!(SubjectQuery::new(101, None, None, None).is_err());
    assert!(SubjectQuery::new(10, None, Some("\nAda"), None).is_err());
    let q = SubjectQuery::new(100, None, Some(" Ada "), Some(SubjectKind::NaturalPerson)).unwrap();
    assert_eq!(q.name(), Some("Ada"));
    assert!(SubjectHistoryQuery::new(1, None).is_ok());
}
#[test]
fn trust_expectations_use_absence_without_a_fake_revision_zero() {
    assert!(CredentialTrustRevision::new(0).is_err());
    assert_eq!(CredentialTrustExpectation::Absent.get(), 0);
    assert_eq!(CredentialTrustExpectation::Absent.next().unwrap().get(), 1);
    assert!(CredentialTrustRevision::new(u32::MAX)
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn review_canonical_order_is_stable_and_duplicate_decisions_reject() {
    let candidate = IdentityCandidateRef::Subject {
        id: CaseSubjectId::from_uuid(Uuid::from_u128(3)),
        revision: SubjectRevision::initial(),
    };
    let decision = IdentityDifferentDecision {
        candidate,
        reason: ParticipantText::new("Different person").unwrap(),
        support: values(subject()).role().role_support().clone(),
    };
    let review = IdentityReviewSubmission {
        directory_stamp: CaseDirectoryStamp(Sha256Digest::from_array([7; 32])),
        different: vec![decision.clone(), decision],
        selection_reason: ParticipantReason::new("Checked source").unwrap(),
    };
    assert!(review.canonical_bytes().is_err());
    let mut unique = review.clone();
    unique.different.pop();
    let bytes = unique.canonical_bytes().unwrap();
    assert!(bytes.starts_with(b"PREV1"));
    unique.selection_reason = ParticipantReason::new("Another explanation").unwrap();
    assert_ne!(bytes, unique.canonical_bytes().unwrap());
}

#[test]
fn new_typed_root_cannot_start_archived() {
    let reference = subject();
    let archived = values(reference).with_directory_status(DirectoryStatus::Archived);
    assert!(ParticipantProposal::new(
        SubjectChange::Keep(reference),
        ParticipantId::new(),
        ParticipantExpectation::Absent,
        archived
    )
    .is_err());
}
