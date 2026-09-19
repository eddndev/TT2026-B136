#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;
use application::{cases::*, deadlines::*};
use deadline_service_support::*;
use deadline_service_support::{prepare_human as prepare, tracked_detail as detail};
use domain::{
    case_administration::{CaseAdministrationValues, CaseRevision},
    cases::CaseMetadata,
    identity::Role,
};

fn newer_administration() -> CurrentCaseAdministration {
    let values =
        CaseAdministrationValues::basic(CaseMetadata::new("Updated title", "REF-NEW").unwrap());
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case_id(),
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(hasher().as_ref(), &values),
        values,
        changed_at: case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: owner(),
            email: "owner@example.com".into(),
        },
    }))
}
fn recapture(preparation: &mut DeadlinePreparation) {
    preparation.administration = newer_administration();
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = preparation.administration.clone();
}
#[test]
fn register_and_correct_accept_new_active_administration_with_same_reviewed_state() {
    for correction in [false, true] {
        let (mut command, mut preparation) = fixture();
        if correction {
            let base = captured();
            command.change = DeadlineChange::Correct {
                expected_revision: base.revision,
                definition: base.definition.clone(),
                reason: evaluation::text("Review the same explicit inputs"),
            };
            preparation.base = Some(base);
        }
        let prepared = prepare(command.clone(), preparation.clone()).unwrap();
        let mut refreshed = preparation.clone();
        recapture(&mut refreshed);
        let committed = detail(&prepare(command.clone(), refreshed).unwrap());
        assert_eq!(committed.receipt.review_digest, prepared.review_digest());
        assert_eq!(
            committed.receipt.submission_digest,
            prepared.submission_digest()
        );
        assert_ne!(committed.receipt.capture_digest, prepared.capture_digest());
        deadline_receipt_matches(hasher().as_ref(), &committed).unwrap();
        let expected = committed.clone();
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(preparation));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(committed));
        let (workflow, _) = service(store, identity(Role::Owner, 2));
        assert_eq!(
            workflow
                .submit(
                    "session",
                    case_id(),
                    human_command(command),
                    prepared.submission_digest()
                )
                .unwrap(),
            expected
        );
    }
}
#[test]
fn attention_and_retirement_reject_replacing_historical_administration_even_with_valid_receipt() {
    let original = captured();
    for retirement in [false, true] {
        let change = if retirement {
            DeadlineChange::Retire {
                expected_revision: original.revision,
                reason: evaluation::text("Retire declaration"),
            }
        } else {
            DeadlineChange::SetAttention {
                expected_revision: original.revision,
                attention: attention(),
                reason: evaluation::text("Record attention"),
            }
        };
        let (command, preparation) = followup(&original, change.clone());
        let prepared = prepare(command.clone(), preparation.clone()).unwrap();
        let mut unexpected = detail(&prepared);
        unexpected.calculation.material.administration = newer_administration();
        unexpected.tracking.as_mut().unwrap().administration = newer_administration();
        unexpected.receipt.capture_digest =
            hasher().hash_bytes(&deadline_capture_bytes(hasher().as_ref(), &unexpected).unwrap());
        deadline_receipt_matches(hasher().as_ref(), &unexpected).unwrap();
        assert_eq!(
            unexpected.receipt.submission_digest,
            prepared.submission_digest()
        );
        assert_ne!(unexpected.receipt.capture_digest, prepared.capture_digest());
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(preparation));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(unexpected));
        let (workflow, _) = service(store, identity(Role::Owner, 2));
        invalid(
            workflow
                .submit(
                    "session",
                    case_id(),
                    human_command(command),
                    prepared.submission_digest(),
                )
                .map(|_| ()),
        );
    }
}

#[test]
fn registration_commit_rejects_administrative_rollback_and_rewriting_the_same_revision() {
    for mutation in 0..4 {
        let (command, mut original) = fixture();
        if mutation != 3 {
            recapture(&mut original);
            if let CurrentCaseAdministration::Recorded(snapshot) = &mut original.administration {
                snapshot.revision = CaseRevision::new(2).unwrap();
            }
            original.resolved.as_mut().unwrap().material.administration =
                original.administration.clone();
        }
        let prepared = prepare(command.clone(), original.clone()).unwrap();
        let mut altered = original.clone();
        match mutation {
            0 => altered.administration = fixture().1.administration,
            1 => altered.administration = newer_administration(),
            2 => {
                let CurrentCaseAdministration::Recorded(snapshot) = &mut altered.administration
                else {
                    unreachable!()
                };
                snapshot.changed_by.email = "another@example.com".into();
            }
            _ => {
                altered.administration = CurrentCaseAdministration::Unrevised(
                    CaseMetadata::new("Different baseline", "OTHER").unwrap(),
                )
            }
        }
        altered.resolved.as_mut().unwrap().material.administration = altered.administration.clone();
        let returned = detail(&prepare(command.clone(), altered).unwrap());
        deadline_receipt_matches(hasher().as_ref(), &returned).unwrap();
        assert_eq!(returned.receipt.review_digest, prepared.review_digest());
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(original));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(returned));
        let (workflow, _) = service(store, identity(Role::Owner, 2));
        invalid(
            workflow
                .submit(
                    "session",
                    case_id(),
                    human_command(command),
                    prepared.submission_digest(),
                )
                .map(|_| ()),
        );
    }
}
