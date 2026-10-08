use application::case_stages::*;
use application::cases::*;
use application::precautionary_hearings::{PrecautionaryContext, PrecautionaryContextMaterial};
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;
use domain::DomainError;
use std::io::Read;
use time::Duration;

use crate::precautionary_context_support::*;

#[test]
fn every_administrative_envelope_field_changes_the_canonical_context() {
    let mut original = changed(trial());
    observed_newer(&mut original);
    let expected = bytes(original.clone());
    for source in [false, true] {
        for field in 0..4 {
            let mut material = original.clone();
            let administration = if source {
                &mut material.stage_administration
            } else {
                &mut material.administration
            };
            match field {
                0 => administration.changed_at += Duration::nanoseconds(1),
                1 => administration.changed_by.email = "other@example.com".into(),
                2 => administration.changed_by.id = UserId::from_uuid(uuid::Uuid::from_u128(99)),
                _ => {
                    administration.revision = CaseRevision::new(if source { 2 } else { 8 }).unwrap()
                }
            }
            if source {
                changed_mut(&mut material).administration_revision =
                    material.stage_administration.revision;
            }
            assert_ne!(
                bytes(material),
                expected,
                "unbound administrative field {field}"
            );
        }
    }
}

#[test]
fn changed_stage_provenance_and_support_metadata_change_the_canonical_context() {
    let original = changed(trial());
    let expected = bytes(original.clone());
    for field in 0..6 {
        let mut material = original.clone();
        let stage = changed_mut(&mut material);
        match field {
            0 => stage.recorded_at += Duration::nanoseconds(1),
            1 => stage.recorded_by.email = "other@example.com".into(),
            2 => stage.recorded_by.id = UserId::from_uuid(uuid::Uuid::from_u128(99)),
            3 => stage.supports[0].name = "other-name.pdf".into(),
            4 => stage.supports[0].format = StageDocumentFormat::Docx,
            _ => stage.stage_revision = CaseStageRevision::new(2).unwrap(),
        }
        assert_ne!(bytes(material), expected, "unbound stage field {field}");
    }
}

#[test]
fn consistent_case_identity_changes_the_canonical_context() {
    let original = changed(trial());
    let mut material = original.clone();
    let other = CaseId::from_uuid(uuid::Uuid::from_u128(99));
    material.case_id = other;
    material.administration.case_id = other;
    material.stage_administration.case_id = other;
    changed_mut(&mut material).case_id = other;
    assert_ne!(bytes(material), bytes(original));
}

struct ConstantHasher;

impl DocumentHasher for ConstantHasher {
    fn hash_bytes(&self, _: &[u8]) -> Sha256Digest {
        Sha256Digest::from_array([42; 32])
    }

    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        Ok(self.hash_bytes(&[]))
    }
}

fn colliding_digest_bytes(mut material: PrecautionaryContextMaterial) -> Vec<u8> {
    material.administration.values_digest =
        case_administration_digest(&ConstantHasher, &material.administration.values);
    material.stage_administration.values_digest =
        case_administration_digest(&ConstantHasher, &material.stage_administration.values);
    let administrative_digest = material.stage_administration.values_digest;
    let stage = changed_mut(&mut material);
    stage.administration_digest = administrative_digest;
    stage.values_digest = case_stage_digest(&ConstantHasher, &stage.values);
    PrecautionaryContext::new(&ConstantHasher, material)
        .unwrap()
        .canonical_bytes()
}

#[test]
fn canonical_context_binds_full_administrative_values_even_when_digests_collide() {
    let mut original = changed(trial());
    observed_newer(&mut original);
    let expected = colliding_digest_bytes(original.clone());
    let profile = administration_values();
    let profile = profile.profile().unwrap();
    for source in [false, true] {
        for field in 0..9 {
            let mut material = original.clone();
            let values = CaseAdministrationValues::new(
                CaseEditableValues::new(
                    CaseMetadata::new(
                        if field == 0 {
                            "Other title"
                        } else {
                            "Penal title"
                        },
                        if field == 1 { "REF-2" } else { "REF-1" },
                    )
                    .unwrap(),
                    Some(
                        PenalCaseProfile::new(
                            if field == 2 { "NUC-2" } else { profile.nuc() },
                            if field == 3 {
                                "Other prosecution"
                            } else {
                                profile.nuc_authority()
                            },
                            if field == 4 {
                                "CJ-2"
                            } else {
                                profile.judicial_case_number()
                            },
                            if field == 5 {
                                "Other court"
                            } else {
                                profile.judicial_authority()
                            },
                            if field == 6 {
                                &["Offense B", "Offense A"]
                            } else {
                                &["Offense A", "Offense B"]
                            },
                            if field == 7 {
                                None
                            } else {
                                profile.general_information()
                            },
                            if field == 8 {
                                None
                            } else {
                                profile.complementary_identifiers()
                            },
                        )
                        .unwrap(),
                    ),
                ),
                CaseAdministrativeStatus::Active,
            );
            if source {
                material.stage_administration.values = values;
            } else {
                material.administration.values = values;
            }
            assert_ne!(
                colliding_digest_bytes(material),
                expected,
                "unbound administrative value {field}"
            );
        }
    }
    let mut closed = original;
    closed.administration.values = closed
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    assert_ne!(colliding_digest_bytes(closed), expected);
}

#[test]
fn canonical_context_binds_full_declared_stage_values_even_when_digests_collide() {
    let original = changed(trial());
    let expected = colliding_digest_bytes(original.clone());
    for field in 0..5 {
        let mut material = original.clone();
        changed_mut(&mut material).values = CaseStageChange::Transition(
            StageTransition::to_trial(
                DeclaredStageTime::instant(if field == 0 {
                    at() - Duration::seconds(1)
                } else {
                    at()
                })
                .unwrap(),
                support(3),
                DeclaredStageTime::instant(
                    at() + Duration::seconds(if field == 1 { 2 } else { 1 }),
                )
                .unwrap(),
                StageCourt::new(if field == 2 {
                    "Other court"
                } else {
                    "Trial court"
                })
                .unwrap(),
                if field == 3 {
                    None
                } else {
                    Some(StageReceiptReference::new("RECEIPT-1").unwrap())
                },
                Some(support(4)),
                if field == 4 {
                    None
                } else {
                    Some(StageNote::new("Declared receipt").unwrap())
                },
            )
            .unwrap(),
        );
        assert_ne!(
            colliding_digest_bytes(material),
            expected,
            "unbound declared value {field}"
        );
    }
}
