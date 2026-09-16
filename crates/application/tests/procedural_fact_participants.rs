#[path = "procedural_fact_participant_support/mod.rs"]
mod support;
use application::{
    procedural_facts::resolve_fact_participants,
    typed_participants::{
        subject_digest, typed_participant_digest, SubjectRevision, TypedParticipantValues,
    },
};
use domain::{crypto::Sha256Digest, participants::DirectoryStatus};
use support::*;

#[test]
fn empty_selection_resolves_without_inventing_people() {
    assert!(
        resolve_fact_participants(&Hasher, case_id(), &selection(&[]), &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn archived_manual_revision_derives_its_exact_public_overview() {
    let detail = manual(7, 2, DirectoryStatus::Archived);
    let digest = detail.values_digest();
    let result =
        resolve_fact_participants(&Hasher, case_id(), &selection(&[(7, 2)]), &[detail]).unwrap();
    assert_eq!(result.len(), 1);
    let projection = &result[0];
    assert_eq!(projection.snapshot.reference, reference(7, 2));
    assert_eq!(projection.snapshot.values_digest, digest);
    assert_eq!(projection.snapshot.status, DirectoryStatus::Archived);
    assert_eq!(projection.snapshot.subject, None);
    assert_eq!(projection.overview.display_name, "Manual name");
    assert_eq!(projection.overview.procedural_role, "Declared role");
    assert_eq!(
        projection.overview.organization.as_deref(),
        Some("Organization")
    );
    assert_eq!(projection.overview.kind, None);
}

#[test]
fn mixed_union_orders_exact_revisions_without_loading_heads() {
    let refs = [(9, 2), (0, u32::MAX), (9, 1), (4, 3)];
    let material = [
        typed(9, 2, false, DirectoryStatus::Archived),
        manual(0, u32::MAX, DirectoryStatus::Archived),
        manual(9, 1, DirectoryStatus::Active),
        typed(4, 3, true, DirectoryStatus::Active),
    ];
    let result =
        resolve_fact_participants(&Hasher, case_id(), &selection(&refs), &material).unwrap();
    let expected = [
        reference(0, u32::MAX),
        reference(4, 3),
        reference(9, 1),
        reference(9, 2),
    ];
    assert_eq!(
        result
            .iter()
            .map(|p| p.snapshot.reference)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn typed_projection_uses_exact_subject_name_and_retains_bound_digest() {
    for institutional in [false, true] {
        let detail = typed(7, 3, institutional, DirectoryStatus::Archived);
        let participant_digest = detail.values_digest();
        let bound = detail.bound_subject.clone().unwrap();
        assert_ne!(participant_digest, bound.values_digest);
        let result =
            resolve_fact_participants(&Hasher, case_id(), &selection(&[(7, 3)]), &[detail])
                .unwrap();
        let projection = &result[0];
        assert_eq!(projection.snapshot.values_digest, participant_digest);
        assert_eq!(projection.snapshot.case_id, case_id());
        assert_eq!(projection.snapshot.reference, reference(7, 3));
        assert_eq!(projection.overview.case_id, case_id());
        assert_eq!(projection.overview.id, reference(7, 3).id);
        assert_eq!(projection.overview.revision, reference(7, 3).revision);
        assert_eq!(
            projection.overview.directory_status,
            DirectoryStatus::Archived
        );
        assert_eq!(
            projection.overview.kind,
            Some(if institutional {
                application::typed_participants::ParticipantKind::TrialCourt
            } else {
                application::typed_participants::ParticipantKind::Defendant
            })
        );
        assert_eq!(
            projection.overview.display_name,
            bound.values.display_name()
        );
        assert_eq!(
            projection.overview.procedural_role,
            if institutional {
                "trial_court"
            } else {
                "defendant"
            }
        );
        assert_eq!(
            projection.overview.organization.as_deref(),
            Some("Role organization")
        );
        assert_eq!(projection.snapshot.status, DirectoryStatus::Archived);
        let subject = projection.snapshot.subject.unwrap();
        assert_eq!(
            (subject.id, subject.revision, subject.values_digest),
            (bound.id, bound.revision, bound.values_digest)
        );
        assert_eq!(projection.overview.subject, Some(subject));
    }
}

#[test]
fn missing_and_unselected_material_are_rejected() {
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[(7, 1)]),
        &[],
    ));
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[]),
        &[manual(7, 1, DirectoryStatus::Active)],
    ));
}

#[test]
fn duplicate_material_cannot_replace_a_missing_exact_reference() {
    let repeated = manual(7, 1, DirectoryStatus::Active);
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[(7, 1), (8, 1)]),
        &[repeated.clone(), repeated],
    ));
}

#[test]
fn more_than_four_material_entries_are_rejected() {
    let material = (1..=5)
        .map(|id| manual(id, 1, DirectoryStatus::Active))
        .collect::<Vec<_>>();
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[(1, 1), (2, 1), (3, 1), (4, 1)]),
        &material,
    ));
}

#[test]
fn a_foreign_case_is_rejected_for_both_revision_families() {
    let mut manual = manual(7, 1, DirectoryStatus::Active);
    manual_mut(&mut manual).case_id = other_case();
    let mut typed = typed(7, 1, false, DirectoryStatus::Active);
    typed_mut(&mut typed).case_id = other_case();
    for detail in [manual, typed] {
        assert_inconsistent(resolve_fact_participants(
            &Hasher,
            case_id(),
            &selection(&[(7, 1)]),
            &[detail],
        ));
    }
}

#[test]
fn wrong_identity_or_revision_is_rejected_with_equal_counts() {
    for (id, revision) in [(8, 1), (7, 2)] {
        assert_inconsistent(resolve_fact_participants(
            &Hasher,
            case_id(),
            &selection(&[(7, 1)]),
            &[manual(id, revision, DirectoryStatus::Active)],
        ));
    }
}

#[test]
fn manual_values_are_rehashed_and_cannot_carry_a_bound_subject() {
    let mut changed = manual(7, 1, DirectoryStatus::Active);
    manual_mut(&mut changed).values_digest = Sha256Digest::from_array([0; 32]);
    let mut linked = manual(7, 1, DirectoryStatus::Active);
    linked.bound_subject = typed(7, 1, false, DirectoryStatus::Active).bound_subject;
    for detail in [changed, linked] {
        assert_inconsistent(resolve_fact_participants(
            &Hasher,
            case_id(),
            &selection(&[(7, 1)]),
            &[detail],
        ));
    }
}

#[test]
fn typed_values_are_rehashed_and_require_their_bound_subject() {
    let mut changed = typed(7, 1, false, DirectoryStatus::Active);
    typed_mut(&mut changed).values_digest = Sha256Digest::from_array([0; 32]);
    let mut unbound = typed(7, 1, false, DirectoryStatus::Active);
    unbound.bound_subject = None;
    for detail in [changed, unbound] {
        assert_inconsistent(resolve_fact_participants(
            &Hasher,
            case_id(),
            &selection(&[(7, 1)]),
            &[detail],
        ));
    }
}

#[test]
fn bound_subject_requires_exact_case_identity_revision_and_claimed_digest() {
    for mutation in 0..4 {
        let mut detail = typed(7, 1, false, DirectoryStatus::Active);
        let subject = detail.bound_subject.as_mut().unwrap();
        match mutation {
            0 => subject.case_id = other_case(),
            1 => subject.id = application::typed_participants::CaseSubjectId::new(),
            2 => subject.revision = SubjectRevision::new(8).unwrap(),
            3 => subject.values_digest = Sha256Digest::from_array([0; 32]),
            _ => unreachable!(),
        }
        assert_inconsistent(resolve_fact_participants(
            &Hasher,
            case_id(),
            &selection(&[(7, 1)]),
            &[detail],
        ));
    }
}

#[test]
fn matching_claimed_subject_digests_do_not_skip_rehashing_subject_values() {
    let mut detail = typed(7, 1, false, DirectoryStatus::Active);
    let subject = detail.bound_subject.as_mut().unwrap();
    let application::typed_participants::SubjectValues::NaturalPerson { name, .. } =
        &mut subject.values
    else {
        panic!("natural person fixture expected")
    };
    *name = application::typed_participants::RepresentedName::Known(
        application::typed_participants::ParticipantText::new("Tampered historical name").unwrap(),
    );
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[(7, 1)]),
        &[detail],
    ));
}

#[test]
fn subject_kind_mismatch_is_rejected_even_with_consistent_hashes() {
    let mut detail = typed(7, 1, false, DirectoryStatus::Active);
    let subject = detail.bound_subject.as_mut().unwrap();
    subject.values = subject_values(true);
    subject.values_digest = subject_digest(&Hasher, &subject.values);
    let digest = subject.values_digest;
    let participant = typed_mut(&mut detail);
    let mut bound = participant.values.subject();
    bound.values_digest = digest;
    participant.values = TypedParticipantValues::new(
        bound,
        participant.values.directory_status(),
        participant.values.role().clone(),
    );
    participant.values_digest = typed_participant_digest(&Hasher, &participant.values);
    assert_inconsistent(resolve_fact_participants(
        &Hasher,
        case_id(),
        &selection(&[(7, 1)]),
        &[detail],
    ));
}
