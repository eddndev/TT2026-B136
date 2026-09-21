mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{cases::*, resource_activities::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use resource_activity_support::*;

#[test]
fn permissions_precede_scoped_reads_and_revocation_prevents_every_protected_query() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let command = captures.link();
    let linked = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        command.clone(),
    );
    let adapter = store(&db);
    let paralegal = db.user("paralegal", true);
    let client = db.user("client", true);
    let outsider = db.user("litigator", false);
    assert_eq!(
        adapter
            .get(
                paralegal,
                db.case,
                captures.resource.id,
                linked.id,
                None,
                db.at
            )
            .unwrap()
            .association,
        linked
    );
    assert_eq!(
        adapter
            .history(
                paralegal,
                db.case,
                captures.resource.id,
                linked.id,
                history_query(),
                db.at
            )
            .unwrap()
            .revisions,
        vec![linked.clone()]
    );
    assert!(matches!(
        adapter.prepare(paralegal, db.case, captures.resource.id, &command),
        Err(ApplicationError::PermissionDenied)
    ));
    let before = business_and_audit(&mut db);
    for actor in [client, outsider] {
        for absent in [false, true] {
            let case = if absent { CaseId::new() } else { db.case };
            let result = adapter.list(actor, case, captures.resource.id, query(None, None), db.at);
            assert!(matches!(
                (actor == client, result),
                (true, Err(ApplicationError::PermissionDenied))
                    | (false, Err(ApplicationError::CaseNotFound))
            ));
        }
    }
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        adapter.get(
            paralegal,
            db.case,
            captures.resource.id,
            linked.id,
            None,
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        adapter.list(
            paralegal,
            db.case,
            captures.resource.id,
            query(None, None),
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        adapter.history(
            paralegal,
            db.case,
            captures.resource.id,
            linked.id,
            history_query(),
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(business_and_audit(&mut db), before);
}

#[test]
fn closed_case_allows_exact_replay_and_reads_but_no_new_link_or_unlink() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let command = captures.link();
    let linked = persist(&workflow, db.case, captures.resource.id, command.clone());
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let before = associations(&mut db);
    assert_eq!(
        workflow
            .submit(
                "session",
                db.case,
                captures.resource.id,
                command.clone(),
                linked.receipt.submission_digest
            )
            .unwrap(),
        linked
    );
    assert_eq!(associations(&mut db), before);
    assert_eq!(
        workflow
            .get("session", db.case, captures.resource.id, linked.id, None)
            .unwrap()
            .association,
        linked
    );
    for mutation in [captures.link(), unlink(&linked, captures.head.revision)] {
        assert!(matches!(
            workflow.prepare("session", db.case, captures.resource.id, mutation),
            Err(ApplicationError::CaseClosed)
        ));
    }
    let mut changed = command;
    changed.association_id = ResourceActivityId::new();
    assert!(matches!(
        workflow.prepare("session", db.case, captures.resource.id, changed),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::OperationConflict
        ))
    ));
    assert_eq!(associations(&mut db), before);
}

#[test]
fn foreign_target_and_substituted_captures_fail_before_any_association_write() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let case = db.case;
    hearing_database_support::complete(&mut db);
    let foreign = hearing_database_support::persist(
        &hearing_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        hearing_database_support::schedule(),
    );
    let adapter = store(&db);
    let before = business_and_audit(&mut db);
    for fault in 0..3 {
        let mut command = captures.link();
        let ResourceActivityChange::Link { selection } = &mut command.change else {
            unreachable!()
        };
        match fault {
            0 => {
                selection.target = ResourceActivityTarget::Hearing {
                    id: foreign.snapshot.id,
                    revision: foreign.snapshot.revision,
                    submission_digest: foreign.snapshot.receipt.submission_digest,
                }
            }
            1 => {
                selection.resource.capture_digest =
                    domain::crypto::Sha256Digest::from_array([9; 32])
            }
            _ => selection.act.as_mut().unwrap().resource_revision = captures.head.revision,
        }
        assert!(
            adapter
                .prepare(db.owner, case, captures.resource.id, &command)
                .is_err(),
            "accepted source substitution {fault}"
        );
        assert_eq!(business_and_audit(&mut db), before);
    }
}
