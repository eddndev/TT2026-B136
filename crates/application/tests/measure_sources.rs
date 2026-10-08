mod measure_source_support;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[path = "measure_source_support/provenance.rs"]
mod provenance_tests;

use application::precautionary_measures::resolve_measure_sources;
use application::typed_participants::{
    CaseSubjectId, ParticipantKind, ParticipantText, RepresentedName, SubjectKind, SubjectRevision,
    SubjectValues, TypedParticipantValues,
};
use domain::crypto::Sha256Digest;
use domain::identity::UserId;
use domain::participants::{DirectoryStatus, ParticipantValues};
use domain::precautionary_measures::MeasureValues;
use measure_source_support::*;
use time::Duration;
use uuid::Uuid;

#[test]
fn unknown_supervision_derives_exact_natural_and_institutional_subject_labels() {
    for institutional in [false, true] {
        let fixture = Fixture::unknown(institutional);
        let result = fixture.resolve().unwrap();
        let source = &fixture.sources.subject;
        assert_eq!(result.subject.case_id, source.case_id);
        assert_eq!(result.subject.id, source.id);
        assert_eq!(result.subject.revision, source.revision);
        assert_eq!(result.subject.display_name, source.values.display_name());
        assert_eq!(
            result.subject.kind,
            if institutional {
                SubjectKind::InstitutionalBody
            } else {
                SubjectKind::NaturalPerson
            }
        );
        assert!(result.supervisor.is_none());
    }
}

#[test]
fn manual_supervisor_derives_labels_without_certifying_a_directory_role() {
    for status in [DirectoryStatus::Active, DirectoryStatus::Archived] {
        let fixture = Fixture::manual(status);
        let result = fixture.resolve().unwrap().supervisor.unwrap();
        assert_eq!(result.snapshot.case_id, case_id());
        assert_eq!(result.snapshot.reference.id, reference(7, 3).id());
        assert_eq!(
            result.snapshot.reference.revision,
            reference(7, 3).revision()
        );
        assert_eq!(
            result.snapshot.values_digest,
            fixture.sources.supervisor.unwrap().values_digest()
        );
        assert_eq!(result.snapshot.status, status);
        assert_eq!(result.snapshot.subject, None);
        assert_eq!(result.overview.display_name, "Historical manual name");
        assert_eq!(result.overview.procedural_role, "Declared role");
        assert_eq!(
            result.overview.organization.as_deref(),
            Some("Organization")
        );
        assert_eq!(result.overview.kind, None);
        assert_eq!(result.overview.directory_status, status);
    }
}

#[test]
fn typed_supervisor_accepts_existing_kinds_and_archived_exact_sources() {
    for institutional in [false, true] {
        for status in [DirectoryStatus::Active, DirectoryStatus::Archived] {
            let fixture = Fixture::typed(institutional, status);
            let result = fixture.resolve().unwrap().supervisor.unwrap();
            let source = fixture.sources.supervisor.unwrap();
            let subject = source.bound_subject.as_ref().unwrap();
            let kind = if institutional {
                ParticipantKind::TrialCourt
            } else {
                ParticipantKind::Defendant
            };
            assert_eq!(result.snapshot.values_digest, source.values_digest());
            assert_eq!(result.snapshot.subject, Some(subject_ref(subject)));
            assert_eq!(result.snapshot.status, status);
            assert_eq!(result.overview.subject, Some(subject_ref(subject)));
            assert_eq!(result.overview.display_name, subject.values.display_name());
            assert_eq!(result.overview.procedural_role, kind.as_str());
            assert_eq!(result.overview.kind, Some(kind));
            assert_eq!(
                result.overview.organization.as_deref(),
                Some("Role organization")
            );
        }
    }
}

#[test]
fn subject_requires_exact_case_identity_revision_and_stored_digest() {
    for mutation in 0..4 {
        let mut fixture = Fixture::unknown(false);
        let subject = &mut fixture.sources.subject;
        match mutation {
            0 => subject.case_id = other_case(),
            1 => subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(61)),
            2 => subject.revision = SubjectRevision::new(8).unwrap(),
            _ => subject.values_digest = Sha256Digest::from_array([0; 32]),
        }
        assert!(fixture.resolve().is_err(), "subject mutation {mutation}");
    }
    let fixture = Fixture::unknown(false);
    assert!(
        resolve_measure_sources(&Hasher, other_case(), &fixture.values, &fixture.sources).is_err()
    );
}

#[test]
fn equal_selected_and_stored_subject_digests_still_require_recomputed_values() {
    let mut fixture = Fixture::unknown(false);
    fixture.sources.subject.values_digest = Sha256Digest::from_array([0; 32]);
    fixture.values = MeasureValues::new(input(&fixture.sources));
    assert!(fixture.resolve().is_err());
    let mut fixture = Fixture::unknown(false);
    fixture.sources.subject.values = subject_values(true);
    assert!(fixture.resolve().is_err());
}

#[test]
fn known_supervision_requires_material_and_unknown_rejects_extra_material() {
    let mut fixture = Fixture::manual(DirectoryStatus::Active);
    fixture.sources.supervisor = None;
    assert!(fixture.resolve().is_err());
    for detail in [
        manual(7, 3, DirectoryStatus::Active),
        typed(7, 3, false, DirectoryStatus::Active),
    ] {
        let mut fixture = Fixture::unknown(false);
        fixture.sources.supervisor = Some(detail);
        assert!(fixture.resolve().is_err());
    }
}

#[test]
fn both_supervisor_families_require_exact_scope_identity_and_revision() {
    for typed_source in [false, true] {
        for mutation in 0..3 {
            let mut fixture = if typed_source {
                Fixture::typed(false, DirectoryStatus::Active)
            } else {
                Fixture::manual(DirectoryStatus::Active)
            };
            let detail = fixture.supervisor_mut();
            let (case, id, revision) = match &mut detail.revision {
                application::typed_participants::ParticipantRevisionSnapshot::Manual(source) => {
                    (&mut source.case_id, &mut source.id, &mut source.revision)
                }
                application::typed_participants::ParticipantRevisionSnapshot::Typed(source) => {
                    (&mut source.case_id, &mut source.id, &mut source.revision)
                }
            };
            match mutation {
                0 => *case = other_case(),
                1 => *id = reference(8, 3).id(),
                _ => *revision = reference(7, 4).revision(),
            }
            assert!(fixture.resolve().is_err(), "supervisor mutation {mutation}");
        }
    }
}

#[test]
fn manual_supervisor_requires_recomputed_values_and_no_bound_subject() {
    for mutation in 0..3 {
        let mut fixture = Fixture::manual(DirectoryStatus::Active);
        let detail = fixture.supervisor_mut();
        match mutation {
            0 => manual_mut(detail).values_digest = Sha256Digest::from_array([0; 32]),
            1 => {
                manual_mut(detail).values = ParticipantValues::new(
                    "Changed name",
                    "Declared role",
                    Some("Organization"),
                    Some("Private legal detail"),
                    DirectoryStatus::Active,
                )
                .unwrap()
            }
            _ => detail.bound_subject = typed(7, 3, false, DirectoryStatus::Active).bound_subject,
        }
        assert!(fixture.resolve().is_err(), "manual mutation {mutation}");
    }
}

#[test]
fn typed_supervisor_requires_recomputed_values_and_bound_subject_material() {
    for mutation in 0..3 {
        let mut fixture = Fixture::typed(false, DirectoryStatus::Active);
        let detail = fixture.supervisor_mut();
        match mutation {
            0 => typed_mut(detail).values_digest = Sha256Digest::from_array([0; 32]),
            1 => {
                let source = typed_mut(detail);
                source.values = TypedParticipantValues::new(
                    source.values.subject(),
                    DirectoryStatus::Archived,
                    source.values.role().clone(),
                );
            }
            _ => detail.bound_subject = None,
        }
        assert!(fixture.resolve().is_err(), "typed mutation {mutation}");
    }
}

#[test]
fn supervisor_bound_subject_requires_exact_reference_and_recomputed_values() {
    for mutation in 0..5 {
        let mut fixture = Fixture::typed(false, DirectoryStatus::Active);
        let subject = fixture.supervisor_mut().bound_subject.as_mut().unwrap();
        match mutation {
            0 => subject.case_id = other_case(),
            1 => subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(51)),
            2 => subject.revision = SubjectRevision::new(8).unwrap(),
            3 => subject.values_digest = Sha256Digest::from_array([0; 32]),
            _ => subject.values = subject_values(true),
        }
        assert!(
            fixture.resolve().is_err(),
            "bound subject mutation {mutation}"
        );
    }
}

#[test]
fn incompatible_typed_supervisor_subject_kind_rejects_even_with_matching_digests() {
    let mut fixture = Fixture::typed(false, DirectoryStatus::Active);
    fixture
        .supervisor_mut()
        .bound_subject
        .as_mut()
        .unwrap()
        .values = subject_values(true);
    rebind_supervisor(fixture.supervisor_mut());
    assert!(fixture.resolve().is_err());
}

#[test]
fn identical_subject_material_can_be_selected_directly_and_bound_to_supervisor() {
    for institutional in [false, true] {
        let fixture = Fixture::shared_subject(institutional);
        let result = fixture.resolve().unwrap();
        assert_eq!(
            result.subject.display_name,
            result.supervisor.unwrap().overview.display_name
        );
    }
}

#[test]
fn same_subject_revision_rejects_contradictory_author_and_time_provenance() {
    for mutation in 0..3 {
        let mut fixture = Fixture::shared_subject(false);
        let subject = fixture.supervisor_mut().bound_subject.as_mut().unwrap();
        match mutation {
            0 => subject.changed_by.id = UserId::from_uuid(Uuid::from_u128(99)),
            1 => subject.changed_by.email = "another@example.test".into(),
            _ => subject.changed_at += Duration::seconds(1),
        }
        assert!(
            fixture.resolve().is_err(),
            "contradictory subject mutation {mutation}"
        );
    }
}

#[test]
fn same_subject_revision_rejects_different_full_values_with_self_consistent_hashes() {
    let mut fixture = Fixture::shared_subject(false);
    let subject = fixture.supervisor_mut().bound_subject.as_mut().unwrap();
    let SubjectValues::NaturalPerson { name, .. } = &mut subject.values else {
        panic!("natural person fixture expected")
    };
    *name = RepresentedName::Known(ParticipantText::new("Different historical person").unwrap());
    rebind_supervisor(fixture.supervisor_mut());
    assert!(fixture.resolve().is_err());
}

#[test]
fn distinct_revisions_of_one_subject_can_retain_different_provenance() {
    let mut fixture = Fixture::shared_subject(false);
    let subject = fixture.supervisor_mut().bound_subject.as_mut().unwrap();
    subject.revision = SubjectRevision::new(8).unwrap();
    subject.changed_by.email = "another@example.test".into();
    subject.changed_at += Duration::seconds(1);
    rebind_supervisor(fixture.supervisor_mut());
    assert!(fixture.resolve().is_ok());
}
