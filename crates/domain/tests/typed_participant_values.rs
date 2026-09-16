use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::participants::DirectoryStatus;
use domain::typed_participants::*;
use uuid::Uuid;

fn support() -> ParticipantEvidenceLocator {
    ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(1)),
            version: DocumentVersion::new(1).unwrap(),
        },
        Sha256Digest::from_bytes(&[7; 32]).unwrap(),
        "page 1",
    )
    .unwrap()
}

#[test]
fn values_reject_original_controls_and_preserve_unicode_and_leading_zeroes() {
    assert!(ParticipantText::<200>::new("\nName").is_err());
    assert!(ParticipantReason::new(" \t").is_err());
    assert_eq!(
        ParticipantText::<2>::new(" \u{e9}X ").unwrap().as_str(),
        "\u{e9}X"
    );
    assert!(ParticipantText::<1>::new("e\u{301}").is_err());
    assert_eq!(
        Curp::new(" abcdef123456abcdef ").unwrap().as_str(),
        "ABCDEF123456ABCDEF"
    );
    assert!(Curp::new("ABC").is_err());
    let license = ProfessionalLicense::new("00123", " Authority ").unwrap();
    assert_eq!(license.number(), "00123");
    assert_eq!(license.issuer(), "Authority");
}

#[test]
fn subject_revision_never_exposes_zero_or_wraps() {
    assert!(SubjectRevision::new(0).is_err());
    assert_eq!(SubjectRevision::initial().get(), 1);
    assert_eq!(SubjectRevision::new(u32::MAX).unwrap().next(), None);
}

#[test]
fn subject_canon_binds_exact_support_and_has_a_finite_maximum() {
    let subject = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Ada").unwrap()),
        Declared::Unknown(ParticipantReason::new("not supplied").unwrap()),
        support(),
    );
    let mut expected = b"SUBJ1\0\0\0\0\0\x03Ada\x01\0\0\0\x0cnot supplied".to_vec();
    expected.extend_from_slice(Uuid::from_u128(1).as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&[7; 32]);
    expected.extend_from_slice(b"\0\0\0\x06page 1");
    assert_eq!(subject.canonical_bytes(), expected);
    let wide = "\u{1f600}";
    let largest = SubjectValues::natural_person(
        RepresentedName::Unidentified {
            label: ParticipantText::new(&wide.repeat(200)).unwrap(),
            reason: ParticipantReason::new(&wide.repeat(500)).unwrap(),
        },
        Declared::Unknown(ParticipantReason::new(&wide.repeat(500)).unwrap()),
        ParticipantEvidenceLocator::new(
            support().reference(),
            support().digest(),
            &wide.repeat(200),
        )
        .unwrap(),
    );
    assert_eq!(largest.canonical_bytes().len(), 5676);
}

#[test]
fn typed_canon_preserves_bound_subject_and_status_only_preserves_role() {
    let subject = SubjectRevisionRef {
        id: CaseSubjectId::from_uuid(Uuid::from_u128(2)),
        revision: SubjectRevision::initial(),
        values_digest: Sha256Digest::from_bytes(&[3; 32]).unwrap(),
    };
    let role = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::ControlJudge(ControlJudgeProfile::new("Court").unwrap()),
        support(),
    )
    .unwrap();
    let values = TypedParticipantValues::new(subject, DirectoryStatus::Active, role);
    let mut expected = b"PART2".to_vec();
    expected.extend_from_slice(Uuid::from_u128(2).as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&[3; 32]);
    expected.extend_from_slice(b"\0\0\0\x05\0\0\0\x05Court");
    expected.extend_from_slice(Uuid::from_u128(1).as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&[7; 32]);
    expected.extend_from_slice(b"\0\0\0\x06page 1");
    assert_eq!(values.canonical_bytes(), expected);
    assert!(values.role().profile().requires_credential());
    let archived = values.with_directory_status(DirectoryStatus::Archived);
    assert_eq!(archived.subject(), values.subject());
    assert_eq!(archived.role(), values.role());
    assert_ne!(archived.canonical_bytes(), values.canonical_bytes());
}

#[test]
fn prosecutor_unknown_values_define_the_largest_typed_canon() {
    let wide = "\u{1f600}";
    let reason = || ParticipantReason::new(&wide.repeat(500)).unwrap();
    let profile = ProsecutorProfile::new(
        Declared::Unknown(reason()),
        Declared::Unknown(reason()),
        Declared::Unknown(reason()),
    );
    let role = ParticipantRoleValues::new(
        Some(&wide.repeat(200)),
        Some(&wide.repeat(160)),
        ParticipantProfile::Prosecutor(profile),
        ParticipantEvidenceLocator::new(
            support().reference(),
            support().digest(),
            &wide.repeat(200),
        )
        .unwrap(),
    )
    .unwrap();
    let values = TypedParticipantValues::new(
        SubjectRevisionRef {
            id: CaseSubjectId::new(),
            revision: SubjectRevision::initial(),
            values_digest: Sha256Digest::from_bytes(&[0; 32]).unwrap(),
        },
        DirectoryStatus::Active,
        role,
    );
    assert_eq!(values.canonical_bytes().len(), 8380);
}
