mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;
use application::{
    cases::*, deadline_profiles::DeadlineProfileCollection, deadlines::*, ApplicationError,
};
use deadline_backend_support::*;
use domain::{cases::CaseMetadata, identity::Role};

#[test]
fn commit_rechecks_current_actor_assignment_and_responsible_eligibility() {
    let Some(mut db) = Fixture::new() else { return };
    let profile = profile(&db);
    let source = source(&db);
    for change in 0..6 {
        let actor = if change < 3 {
            db.user("litigator", true)
        } else {
            db.owner
        };
        let responsible = if change < 3 {
            db.owner
        } else {
            db.user("paralegal", true)
        };
        let mut command = command(&db, &profile, &source);
        definition_mut(&mut command).responsible = responsible;
        let pending = prepared_legacy(&db, actor, &command);
        let target = if change < 3 { actor } else { responsible };
        match change {
            0 | 3 => {
                db.admin
                    .execute(
                        "UPDATE users SET active=false WHERE id=$1",
                        &[&target.as_uuid()],
                    )
                    .unwrap();
            }
            1 | 4 => {
                db.admin
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&db.case.as_uuid(), &target.as_uuid()],
                    )
                    .unwrap();
            }
            _ => {
                db.admin
                    .execute(
                        "UPDATE users SET role='client' WHERE id=$1",
                        &[&target.as_uuid()],
                    )
                    .unwrap();
            }
        }
        let before = snapshot(&mut db);
        let result = store(&db).commit(actor, pending);
        match change {
            0 => assert!(matches!(result, Err(ApplicationError::InvalidSession))),
            1 => assert!(matches!(result, Err(ApplicationError::CaseNotFound))),
            2 => assert!(matches!(result, Err(ApplicationError::PermissionDenied))),
            _ => assert!(matches!(
                result,
                Err(ApplicationError::Deadline(
                    DeadlineError::ResponsibleUnavailable
                ))
            )),
        }
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn source_or_profile_head_changes_reject_the_prepared_review_without_partial_writes() {
    let Some(mut db) = Fixture::new() else { return };
    for change_profile in [false, true] {
        let profile = profile(&db);
        let source = source(&db);
        let command = command(&db, &profile, &source);
        let pending = prepared_legacy(&db, db.owner, &command);
        if change_profile {
            let profiles =
                &crate::deadline_profile_database_support::service(&db, db.owner, Role::Owner);
            crate::deadline_profile_database_support::persist(
                profiles,
                DeadlineProfileCollection::ForCase(db.case),
                crate::deadline_profile_database_support::replace(&profile),
            );
        } else {
            let facts =
                &crate::procedural_fact_backend_support::service(&db, db.owner, Role::Owner);
            crate::procedural_fact_backend_support::persist(
                facts,
                db.case,
                crate::procedural_fact_backend_support::correct(&source),
            );
        }
        let before = snapshot(&mut db);
        assert!(store(&db).commit(db.owner, pending).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn only_one_prepared_correction_can_advance_the_same_base_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist_legacy(&db, db.owner, setup(&db));
    let left = prepared_legacy(&db, db.owner, &correct(&first));
    let right = prepared_legacy(&db, db.owner, &correct(&first));
    let second = store(&db).commit(db.owner, left).unwrap();
    assert_eq!(second.revision.get(), 2);
    let before = snapshot(&mut db);
    assert!(matches!(
        store(&db).commit(db.owner, right),
        Err(ApplicationError::Deadline(DeadlineError::RevisionConflict))
    ));
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        workflow.get("session", db.case, first.id, None).unwrap(),
        second
    );
}

#[test]
fn active_administration_advances_capture_while_attention_keeps_historical_calculation() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist_legacy(&db, db.owner, setup(&db));
    let pending = prepared_legacy(&db, db.owner, &correct(&first));
    let reviewed = pending.review_digest();
    let old_capture = pending.capture_digest();
    let revised = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Unrevised,
            CaseEditableValues::new(
                CaseMetadata::new("Changed active administration", "REF-NEW").unwrap(),
                None,
            ),
            db.at,
        )
        .unwrap();
    let second = store(&db).commit(db.owner, pending).unwrap();
    assert_eq!(second.receipt.review_digest, reviewed);
    assert_ne!(second.receipt.capture_digest, old_capture);
    assert_eq!(
        second.calculation.material.administration,
        revised.administration
    );
    assert_eq!(
        workflow
            .get("session", db.case, first.id, Some(first.revision))
            .unwrap(),
        first
    );
    let pending = prepared_legacy(&db, db.owner, &attention(&second));
    let captured = pending.capture_digest();
    db.store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseEditableValues::new(
                CaseMetadata::new("Later active administration", "REF-LATER").unwrap(),
                None,
            ),
            db.at,
        )
        .unwrap();
    let third = store(&db).commit(db.owner, pending).unwrap();
    assert_eq!(third.receipt.capture_digest, captured);
    assert_eq!(third.calculation, second.calculation);
    assert_eq!(
        workflow
            .get("session", db.case, third.id, Some(third.revision))
            .unwrap(),
        third
    );
}
