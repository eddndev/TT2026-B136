mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
#[path = "resource_deadline_database_support/concurrency.rs"]
mod resource_deadline_concurrency;
mod resource_deadline_database_support;
use application::{resource_activities::*, resource_deadlines::*, ApplicationError};
use domain::{crypto::Sha256Digest, identity::Role};
use resource_deadline_database_support::*;

#[test]
fn registration_and_link_share_one_operation_and_exact_replay() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    assert_eq!(
        pair_rows(&mut db),
        serde_json::json!({"deadlines":[],"associations":[]})
    );
    let result = workflow
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(result.deadline.revision.get(), 1);
    assert_eq!(result.association.revision.get(), 1);
    assert_eq!(
        result.deadline.receipt.operation_id.as_uuid(),
        result.association.receipt.operation_id.as_uuid()
    );
    assert_eq!(
        result.association.sources.target,
        ResourceActivityTargetDetail::Deadline(Box::new(result.deadline.clone()))
    );
    assert_eq!(result.association.selection.resource, command.resource);
    assert_eq!(result.association.selection.act, command.act);
    let original = pair_rows(&mut db);
    captures.archive(&db);
    let replay = workflow
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(replay, result);
    assert_eq!(pair_rows(&mut db), original);
    let mut changed = command;
    changed.association_id = ResourceActivityId::new();
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            captures.resource.id,
            changed,
            draft.submission_digest
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::OperationConflict
        ))
    ));
}

#[test]
fn association_audit_failure_rolls_back_deadline_link_events_and_mutation_audits() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_contextual_link() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected second mutation audit failure'; END; $$; CREATE TRIGGER reject_contextual_link BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='resource_activity.link') EXECUTE FUNCTION reject_contextual_link()").unwrap();
    let before = atomic_rows(&mut db);
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            draft.submission_digest
        ),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(atomic_rows(&mut db), before);
    db.admin.batch_execute("DROP TRIGGER reject_contextual_link ON audit_events; DROP FUNCTION reject_contextual_link()").unwrap();
    workflow
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command,
            draft.submission_digest,
        )
        .unwrap();
}

#[test]
fn preexisting_ordinary_deadline_is_never_adopted_as_contextual_replay() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    deadline_backend_support::persist(
        &deadline_backend_support::service(&db, db.owner, Role::Owner),
        db.case,
        command.deadline.clone(),
    );
    let before = pair_rows(&mut db);
    assert!(matches!(
        service(&db, db.owner, Role::Owner).prepare(
            "session",
            db.case,
            captures.resource.id,
            command
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::OperationConflict
        ))
    ));
    assert_eq!(pair_rows(&mut db), before);
}

#[test]
fn roles_membership_and_inactive_accounts_are_checked_durably() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    for role in [Role::Paralegal, Role::Client] {
        let user = db.user(role.as_str(), true);
        assert!(matches!(
            service(&db, user, role).prepare(
                "session",
                db.case,
                captures.resource.id,
                command.clone()
            ),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let actor = db.user("litigator", true);
    let workflow = service(&db, actor, Role::Litigator);
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    assert!(workflow
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            draft.submission_digest
        )
        .is_err());
    db.user("owner", false);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",&[&db.owner.as_uuid()]).unwrap();
    assert!(matches!(
        service(&db, db.owner, Role::Owner).prepare(
            "session",
            db.case,
            captures.resource.id,
            command
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(
        pair_rows(&mut db),
        serde_json::json!({"deadlines":[],"associations":[]})
    );
}

#[test]
fn changed_resource_head_or_review_digest_leaves_no_half_created_deadline() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            Sha256Digest::from_array([8; 32])
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::SubmissionMismatch
        ))
    ));
    captures.archive(&db);
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            captures.resource.id,
            command,
            draft.submission_digest
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::ResourceRevisionConflict
        ))
    ));
    assert_eq!(
        pair_rows(&mut db),
        serde_json::json!({"deadlines":[],"associations":[]})
    );
}
