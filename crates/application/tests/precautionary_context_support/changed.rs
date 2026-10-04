use application::case_stages::*;
use application::cases::{case_administration_digest, CaseAdministrativeStatus, CaseRevision};
use application::precautionary_hearings::PrecautionaryContext;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use time::Duration;

use crate::precautionary_context_support::*;

#[test]
fn retained_stage_administration_must_have_been_active() {
    let mut material = changed(trial());
    material.stage_administration.values = material
        .stage_administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    material.stage_administration.values_digest =
        case_administration_digest(&Hasher, &material.stage_administration.values);
    material.administration = material.stage_administration.clone();
    changed_mut(&mut material).administration_digest = material.stage_administration.values_digest;
    rejected(material);
}

#[test]
fn changed_stage_requires_exact_parent_administration_revision_and_digest() {
    for revision in [false, true] {
        let mut material = changed(trial());
        if revision {
            changed_mut(&mut material).administration_revision = CaseRevision::new(2).unwrap();
        } else {
            changed_mut(&mut material).administration_digest = Sha256Digest::from_array([99; 32]);
        }
        rejected(material);
    }
}

#[test]
fn changed_stage_values_digest_is_recomputed() {
    let mut material = changed(trial());
    changed_mut(&mut material).values_digest = Sha256Digest::from_array([99; 32]);
    rejected(material);
}

#[test]
fn adoption_requires_first_stage_revision_without_a_predecessor() {
    for stage in [
        CaseStage::Investigation,
        CaseStage::Intermediate,
        CaseStage::Trial,
    ] {
        let mut material = changed(adoption(stage));
        changed_mut(&mut material).stage_revision = CaseStageRevision::new(2).unwrap();
        rejected(material);
        for predecessor in [
            CaseStage::Investigation,
            CaseStage::Intermediate,
            CaseStage::Trial,
        ] {
            let mut material = changed(adoption(stage));
            changed_mut(&mut material).from_stage = Some(predecessor);
            rejected(material);
        }
    }
}

#[test]
fn transitions_require_their_predecessor_and_a_possible_stage_revision() {
    for (values, expected, allowed) in [
        (intermediate(), CaseStage::Investigation, vec![2]),
        (trial(), CaseStage::Intermediate, vec![2, 3]),
    ] {
        for revision in [1, 2, 3, 4, u32::MAX] {
            let mut material = changed(values.clone());
            changed_mut(&mut material).stage_revision = CaseStageRevision::new(revision).unwrap();
            assert_eq!(
                PrecautionaryContext::new(&Hasher, material).is_ok(),
                allowed.contains(&revision),
                "unexpected acceptance for revision {revision}"
            );
        }
        for predecessor in [
            None,
            Some(CaseStage::Investigation),
            Some(CaseStage::Intermediate),
            Some(CaseStage::Trial),
        ] {
            let mut material = changed(values.clone());
            changed_mut(&mut material).from_stage = predecessor;
            assert_eq!(
                PrecautionaryContext::new(&Hasher, material).is_ok(),
                predecessor == Some(expected)
            );
        }
    }
}

#[test]
fn selected_supports_require_exact_references_digests_count_and_first_role_order() {
    for mutation in 0..8 {
        let mut material = changed(trial());
        let supports = &mut changed_mut(&mut material).supports;
        match mutation {
            0 => supports.clear(),
            1 => {
                supports.pop();
            }
            2 => supports.push(supports[0].clone()),
            3 => supports.swap(0, 1),
            4 => supports[0] = supports[1].clone(),
            5 => supports[0].reference.id = DocumentId::from_uuid(uuid::Uuid::from_u128(99)),
            6 => supports[0].reference.version = DocumentVersion::new(4).unwrap(),
            _ => supports[0].digest = Sha256Digest::from_array([99; 32]),
        }
        rejected(material);
    }
}

#[test]
fn repeated_support_roles_preserve_one_exact_documentary_snapshot() {
    let values = CaseStageChange::Transition(
        StageTransition::to_trial(
            DeclaredStageTime::instant(at()).unwrap(),
            support(3),
            DeclaredStageTime::instant(at()).unwrap(),
            StageCourt::new("Trial court").unwrap(),
            None,
            Some(support(3)),
            None,
        )
        .unwrap(),
    );
    let mut material = changed(values);
    assert_eq!(changed_mut(&mut material).supports.len(), 1);
    assert!(PrecautionaryContext::new(&Hasher, material.clone()).is_ok());
    let duplicate = changed_mut(&mut material).supports[0].clone();
    changed_mut(&mut material).supports.push(duplicate);
    rejected(material);
}

#[test]
fn retained_support_names_must_be_safe_archive_entry_names() {
    for name in [
        "",
        "../escape.pdf",
        "/absolute.pdf",
        "folder/../escape.pdf",
        "bad\\name.pdf",
        "bad\0name.pdf",
    ] {
        let mut material = changed(adoption(CaseStage::Trial));
        changed_mut(&mut material).supports[0].name = name.into();
        rejected(material);
    }
}

#[test]
fn pdf_and_docx_support_formats_and_their_original_policy_are_retained() {
    let mut material = changed(trial());
    changed_mut(&mut material).supports[1].name = "receipt.docx".into();
    changed_mut(&mut material).supports[1].format = StageDocumentFormat::Docx;
    let context = PrecautionaryContext::new(&Hasher, material.clone()).unwrap();
    assert_eq!(context.material(), &material);
}

#[test]
fn declared_stage_acts_cannot_be_in_the_future_at_original_capture() {
    for values in [adoption(CaseStage::Trial), intermediate(), trial()] {
        let mut material = changed(values);
        material.administration.changed_at = at() - Duration::seconds(20);
        material.stage_administration = material.administration.clone();
        changed_mut(&mut material).recorded_at = at() - Duration::seconds(10);
        rejected(material);
    }
}

#[test]
fn observed_administration_may_have_been_recorded_before_the_stage_capture() {
    let mut material = changed(trial());
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = at() + Duration::seconds(1);
    assert!(PrecautionaryContext::new(&Hasher, material).is_ok());
}
