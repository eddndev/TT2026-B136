mod precautionary_participant_support;

use application::precautionary_hearings::resolve_precautionary_participants;
use application::typed_participants::{
    subject_digest, typed_participant_digest, CaseSubjectId, ParticipantKind, ParticipantText,
    RepresentedName, SubjectRevision, SubjectValues, TypedParticipantValues,
};
use domain::crypto::Sha256Digest;
use domain::participants::{DirectoryStatus, ParticipantValues};
use domain::precautionary_hearings::PrecautionaryHearingValues;
use precautionary_participant_support::*;
use uuid::Uuid;

#[test]
fn empty_selection_resolves_without_inventing_participants() {
    let result = resolve_precautionary_participants(&Hasher, case_id(), &values(&[]), &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn active_and_archived_manual_captures_derive_their_exact_overviews() {
    for status in [DirectoryStatus::Active, DirectoryStatus::Archived] {
        let detail = manual(7, 2, status);
        let digest = detail.values_digest();
        let result =
            resolve_precautionary_participants(&Hasher, case_id(), &values(&[(7, 2)]), &[detail])
                .unwrap();
        assert_eq!(result.len(), 1);
        let projection = &result[0];
        assert_eq!(projection.snapshot.case_id, case_id());
        assert_eq!(projection.snapshot.reference.id, reference(7, 2).id());
        assert_eq!(
            projection.snapshot.reference.revision,
            reference(7, 2).revision()
        );
        assert_eq!(projection.snapshot.values_digest, digest);
        assert_eq!(projection.snapshot.status, status);
        assert_eq!(projection.snapshot.subject, None);
        assert_eq!(projection.overview.case_id, case_id());
        assert_eq!(projection.overview.id, reference(7, 2).id());
        assert_eq!(projection.overview.revision, reference(7, 2).revision());
        assert_eq!(projection.overview.display_name, "Historical manual name");
        assert_eq!(projection.overview.procedural_role, "Declared role");
        assert_eq!(
            projection.overview.organization.as_deref(),
            Some("Organization")
        );
        assert_eq!(projection.overview.directory_status, status);
        assert_eq!(projection.overview.kind, None);
        assert_eq!(projection.overview.subject, None);
    }
}

#[test]
fn typed_captures_derive_names_and_exact_subject_bindings_from_historical_material() {
    for institutional in [false, true] {
        let detail = typed(7, 3, institutional, DirectoryStatus::Archived);
        let digest = detail.values_digest();
        let subject = detail.bound_subject.clone().unwrap();
        let result =
            resolve_precautionary_participants(&Hasher, case_id(), &values(&[(7, 3)]), &[detail])
                .unwrap();
        let projection = &result[0];
        let bound = projection.snapshot.subject.unwrap();
        assert_eq!(projection.snapshot.values_digest, digest);
        assert_eq!(projection.snapshot.status, DirectoryStatus::Archived);
        assert_eq!(
            (bound.id, bound.revision, bound.values_digest),
            (subject.id, subject.revision, subject.values_digest)
        );
        assert_eq!(projection.overview.subject, Some(bound));
        assert_eq!(
            projection.overview.display_name,
            subject.values.display_name()
        );
        assert_eq!(
            projection.overview.organization.as_deref(),
            Some("Role organization")
        );
        assert_eq!(
            projection.overview.directory_status,
            DirectoryStatus::Archived
        );
        let kind = if institutional {
            ParticipantKind::TrialCourt
        } else {
            ParticipantKind::Defendant
        };
        assert_eq!(projection.overview.kind, Some(kind));
        assert_eq!(projection.overview.procedural_role, kind.as_str());
    }
}

#[test]
fn reversed_mixed_material_is_normalized_by_selected_participant_uuid() {
    let selected = values(&[(9, 2), (4, 3), (0, u32::MAX)]);
    let mut material = vec![
        typed(9, 2, false, DirectoryStatus::Archived),
        manual(0, u32::MAX, DirectoryStatus::Archived),
        typed(4, 3, true, DirectoryStatus::Active),
    ];
    let first =
        resolve_precautionary_participants(&Hasher, case_id(), &selected, &material).unwrap();
    material.reverse();
    let reversed =
        resolve_precautionary_participants(&Hasher, case_id(), &selected, &material).unwrap();
    assert_eq!(first, reversed);
    assert_eq!(
        first
            .iter()
            .map(|item| (item.snapshot.reference.id, item.snapshot.reference.revision))
            .collect::<Vec<_>>(),
        selected
            .participants()
            .iter()
            .map(|item| (item.id(), item.revision()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn thirty_two_distinct_participants_resolve_without_the_fact_collection_limit() {
    let references = (1..=32).map(|id| (id, 1)).collect::<Vec<_>>();
    let selected = values(&references);
    let mut material = (1..=32)
        .map(|id| manual(id, 1, DirectoryStatus::Active))
        .collect::<Vec<_>>();
    material.reverse();
    let result =
        resolve_precautionary_participants(&Hasher, case_id(), &selected, &material).unwrap();
    assert_eq!(result.len(), 32);
    for (projection, expected) in result.iter().zip(selected.participants()) {
        assert_eq!(projection.snapshot.reference.id, expected.id());
        assert_eq!(projection.snapshot.reference.revision, expected.revision());
    }
    material.push(manual(33, 1, DirectoryStatus::Active));
    assert!(resolve_precautionary_participants(&Hasher, case_id(), &selected, &material).is_err());
    let mut oversized = hearing_input(&references);
    oversized.participants.push(reference(33, 1));
    assert!(PrecautionaryHearingValues::new(oversized).is_err());
}

#[test]
fn missing_excess_and_unselected_material_are_rejected() {
    let material = manual(7, 1, DirectoryStatus::Active);
    assert_rejected(&[(7, 1)], &[]);
    assert_rejected(&[], std::slice::from_ref(&material));
    assert_rejected(
        &[(7, 1)],
        &[material, manual(8, 1, DirectoryStatus::Active)],
    );
}

#[test]
fn duplicate_identity_material_cannot_replace_another_selected_participant() {
    for second_revision in [1, 2] {
        let material = [
            manual(7, 1, DirectoryStatus::Active),
            manual(7, second_revision, DirectoryStatus::Active),
        ];
        assert_rejected(&[(7, 1), (8, 1)], &material);
    }
}

#[test]
fn both_participant_families_reject_foreign_case_material() {
    let mut foreign_manual = manual(7, 1, DirectoryStatus::Active);
    manual_mut(&mut foreign_manual).case_id = other_case();
    let mut foreign_typed = typed(7, 1, false, DirectoryStatus::Active);
    typed_mut(&mut foreign_typed).case_id = other_case();
    for detail in [foreign_manual, foreign_typed] {
        assert_rejected(&[(7, 1)], &[detail]);
    }
}

#[test]
fn both_participant_families_require_the_exact_selected_identity_and_revision() {
    for (id, revision) in [(8, 1), (7, 2)] {
        for detail in [
            manual(id, revision, DirectoryStatus::Active),
            typed(id, revision, false, DirectoryStatus::Active),
        ] {
            assert_rejected(&[(7, 1)], &[detail]);
        }
    }
}

#[test]
fn manual_captures_require_recomputed_values_and_no_bound_subject() {
    let mut wrong_digest = manual(7, 1, DirectoryStatus::Active);
    manual_mut(&mut wrong_digest).values_digest = Sha256Digest::from_array([0; 32]);
    let mut unexpected_subject = manual(7, 1, DirectoryStatus::Active);
    unexpected_subject.bound_subject = typed(7, 1, false, DirectoryStatus::Active).bound_subject;
    let mut changed_values = manual(7, 1, DirectoryStatus::Active);
    manual_mut(&mut changed_values).values = ParticipantValues::new(
        "Changed name",
        "Declared role",
        Some("Organization"),
        Some("Private legal detail"),
        DirectoryStatus::Active,
    )
    .unwrap();
    for detail in [wrong_digest, unexpected_subject, changed_values] {
        assert_rejected(&[(7, 1)], &[detail]);
    }
}

#[test]
fn typed_captures_require_recomputed_values_and_their_bound_subject() {
    let mut wrong_digest = typed(7, 1, false, DirectoryStatus::Active);
    typed_mut(&mut wrong_digest).values_digest = Sha256Digest::from_array([0; 32]);
    let mut unbound = typed(7, 1, false, DirectoryStatus::Active);
    unbound.bound_subject = None;
    let mut changed_values = typed(7, 1, false, DirectoryStatus::Active);
    let snapshot = typed_mut(&mut changed_values);
    snapshot.values = TypedParticipantValues::new(
        snapshot.values.subject(),
        DirectoryStatus::Archived,
        snapshot.values.role().clone(),
    );
    for detail in [wrong_digest, unbound, changed_values] {
        assert_rejected(&[(7, 1)], &[detail]);
    }
}

#[test]
fn bound_subject_must_match_exact_case_identity_revision_and_digest() {
    for mutation in 0..4 {
        let mut detail = typed(7, 1, false, DirectoryStatus::Active);
        let subject = detail.bound_subject.as_mut().unwrap();
        match mutation {
            0 => subject.case_id = other_case(),
            1 => subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(51)),
            2 => subject.revision = SubjectRevision::new(8).unwrap(),
            _ => subject.values_digest = Sha256Digest::from_array([0; 32]),
        }
        assert_rejected(&[(7, 1)], &[detail]);
    }
}

#[test]
fn matching_claimed_subject_digest_cannot_hide_changed_historical_values() {
    let mut detail = typed(7, 1, false, DirectoryStatus::Active);
    let subject = detail.bound_subject.as_mut().unwrap();
    let SubjectValues::NaturalPerson { name, .. } = &mut subject.values else {
        panic!("natural person fixture expected")
    };
    *name = RepresentedName::Known(ParticipantText::new("Changed historical person").unwrap());
    assert_rejected(&[(7, 1)], &[detail]);
}

#[test]
fn incompatible_subject_kind_rejects_even_when_all_values_hashes_match() {
    let mut detail = typed(7, 1, false, DirectoryStatus::Active);
    let subject = detail.bound_subject.as_mut().unwrap();
    subject.values = subject_values(true);
    subject.values_digest = subject_digest(&Hasher, &subject.values);
    let digest = subject.values_digest;
    let snapshot = typed_mut(&mut detail);
    let mut bound = snapshot.values.subject();
    bound.values_digest = digest;
    snapshot.values = TypedParticipantValues::new(
        bound,
        snapshot.values.directory_status(),
        snapshot.values.role().clone(),
    );
    snapshot.values_digest = typed_participant_digest(&Hasher, &snapshot.values);
    assert_rejected(&[(7, 1)], &[detail]);
}
