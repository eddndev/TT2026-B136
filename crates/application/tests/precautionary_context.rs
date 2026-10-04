#[path = "precautionary_context_support/binding.rs"]
mod binding;
#[path = "precautionary_context_support/changed.rs"]
mod changed_validation;
mod precautionary_context_support;
#[path = "precautionary_context_support/provenance.rs"]
mod provenance;
#[path = "precautionary_context_support/vectors.rs"]
mod vectors;

use application::case_stages::*;
use application::cases::*;
use application::precautionary_hearings::PrecautionaryContext;
use domain::cases::CaseId;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;
use precautionary_context_support::*;
use time::Duration;

#[test]
fn complete_initial_context_preserves_its_exact_material_and_stable_encoding() {
    let material = initial();
    let context = PrecautionaryContext::new(&Hasher, material.clone()).unwrap();
    assert_eq!(context.material(), &material);
    assert_eq!(context.canonical_bytes(), bytes(material));
    assert!(context.canonical_bytes().starts_with(b"PCTX1"));
    assert_eq!(
        context.digest(&Hasher),
        Hasher.hash_bytes(&context.canonical_bytes())
    );
}

#[test]
fn all_recorded_stages_can_be_reconstructed_without_inferring_hearing_eligibility() {
    for values in [
        adoption(CaseStage::Investigation),
        adoption(CaseStage::Intermediate),
        adoption(CaseStage::Trial),
        intermediate(),
        trial(),
    ] {
        let material = changed(values);
        let context = PrecautionaryContext::new(&Hasher, material.clone()).unwrap();
        assert_eq!(context.material(), &material);
    }
}

#[test]
fn observed_newer_administration_preserves_exact_stage_administration() {
    let mut material = changed(trial());
    let retained = material.stage_administration.clone();
    observed_newer(&mut material);
    material.administration.values = material
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    material.administration.values_digest =
        case_administration_digest(&Hasher, &material.administration.values);
    let context = PrecautionaryContext::new(&Hasher, material.clone()).unwrap();
    assert_eq!(context.material().stage_administration, retained);
    assert_eq!(context.material().administration, material.administration);
}

#[test]
fn historical_reconstruction_accepts_active_and_closed_observed_administration() {
    for status in [
        CaseAdministrativeStatus::Active,
        CaseAdministrativeStatus::Closed,
    ] {
        let mut material = initial();
        observed_newer(&mut material);
        material.administration.values = material.administration.values.with_status(status);
        material.administration.values_digest =
            case_administration_digest(&Hasher, &material.administration.values);
        assert!(PrecautionaryContext::new(&Hasher, material).is_ok());
    }
}

#[test]
fn incomplete_penal_profiles_are_rejected_in_both_administrative_snapshots() {
    for observed in [false, true] {
        let mut material = changed(trial());
        observed_newer(&mut material);
        let administration = if observed {
            &mut material.administration
        } else {
            &mut material.stage_administration
        };
        administration.values =
            CaseAdministrationValues::basic(administration.values.metadata().clone());
        administration.values_digest = case_administration_digest(&Hasher, &administration.values);
        if !observed {
            changed_mut(&mut material).administration_digest =
                material.stage_administration.values_digest;
        }
        rejected(material);
    }
}

#[test]
fn case_identity_must_match_every_captured_source() {
    for location in 0..4 {
        let mut material = initial();
        let other = CaseId::from_uuid(uuid::Uuid::from_u128(99));
        match location {
            0 => material.case_id = other,
            1 => material.administration.case_id = other,
            2 => material.stage_administration.case_id = other,
            _ => match &mut material.stage {
                CaseStageEntry::Initial(stage) => stage.case_id = other,
                _ => unreachable!(),
            },
        }
        rejected(material);
    }
    let mut material = changed(trial());
    changed_mut(&mut material).case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99));
    rejected(material);
}

#[test]
fn administration_values_digests_are_recomputed_for_both_snapshots() {
    for observed in [false, true] {
        let mut material = changed(trial());
        observed_newer(&mut material);
        if observed {
            material.administration.values_digest = Sha256Digest::from_array([99; 32]);
        } else {
            material.stage_administration.values_digest = Sha256Digest::from_array([99; 32]);
            changed_mut(&mut material).administration_digest =
                material.stage_administration.values_digest;
        }
        rejected(material);
    }
}

#[test]
fn observed_administration_cannot_precede_stage_administration_revision() {
    let mut material = changed(intermediate());
    material.stage_administration.revision = CaseRevision::new(2).unwrap();
    changed_mut(&mut material).administration_revision = CaseRevision::new(2).unwrap();
    rejected(material);
}

#[test]
fn equal_administrative_revisions_require_the_entire_snapshot_to_match() {
    for field in 0..4 {
        let mut material = changed(trial());
        match field {
            0 => material.administration.changed_at += Duration::nanoseconds(1),
            1 => material.administration.changed_by.email = "other@example.com".into(),
            2 => {
                material.administration.changed_by.id =
                    UserId::from_uuid(uuid::Uuid::from_u128(99));
            }
            _ => {
                material.administration.values = material
                    .administration
                    .values
                    .with_status(CaseAdministrativeStatus::Closed);
                material.administration.values_digest =
                    case_administration_digest(&Hasher, &material.administration.values);
            }
        }
        rejected(material);
    }
}

#[test]
fn initial_registration_requires_first_stage_and_administration_revisions() {
    for stage_counter in [false, true] {
        let mut material = initial();
        let CaseStageEntry::Initial(stage) = &mut material.stage else {
            unreachable!()
        };
        if stage_counter {
            stage.stage_revision = CaseStageRevision::new(2).unwrap();
        } else {
            stage.administration_revision = CaseRevision::new(2).unwrap();
            material.administration.revision = stage.administration_revision;
            material.stage_administration.revision = stage.administration_revision;
        }
        rejected(material);
    }
}

#[test]
fn initial_registration_requires_exact_stage_administration_digest_actor_and_time() {
    for field in 0..4 {
        let mut material = initial();
        let CaseStageEntry::Initial(stage) = &mut material.stage else {
            unreachable!()
        };
        match field {
            0 => stage.administration_digest = Sha256Digest::from_array([99; 32]),
            1 => stage.recorded_at += Duration::nanoseconds(1),
            2 => stage.recorded_by.email = "other@example.com".into(),
            _ => stage.recorded_by.id = UserId::from_uuid(uuid::Uuid::from_u128(99)),
        }
        rejected(material);
    }
}
