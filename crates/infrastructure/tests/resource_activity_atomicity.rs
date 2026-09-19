mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{resource_activities::*, ApplicationError};
use domain::identity::Role;
use resource_activity_support::*;
use std::sync::Arc;

#[test]
fn rejected_audit_rolls_back_association_and_does_not_reserve_operation() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let command = captures.link();
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_association_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected association audit failure'; END; $$; CREATE TRIGGER reject_association_audit BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='resource_activity.link') EXECUTE FUNCTION reject_association_audit()").unwrap();
    let before = business_and_audit(&mut db);
    let independent = independent_rows(&mut db);
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
    assert_eq!(business_and_audit(&mut db), before);
    assert_eq!(independent_rows(&mut db), independent);
    db.admin.batch_execute("DROP TRIGGER reject_association_audit ON audit_events; DROP FUNCTION reject_association_audit()").unwrap();
    assert_eq!(
        workflow
            .submit(
                "session",
                db.case,
                captures.resource.id,
                command.clone(),
                draft.submission_digest
            )
            .unwrap()
            .receipt
            .operation_id,
        command.operation_id
    );
}

#[test]
fn commit_rechecks_membership_and_resource_head_after_successful_preparation() {
    for revoke in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let captures = Captures::new(&mut db);
        let actor = db.user("litigator", true);
        let command = captures.link();
        let draft = service(&db, actor, Role::Litigator)
            .prepare("session", db.case, captures.resource.id, command.clone())
            .unwrap();
        let url = db.admin_url.clone();
        let resources = procedural_resource_support::service(&db, db.owner, Role::Owner);
        let resource = captures.resource.id;
        let head = captures.head.revision;
        let case = db.case;
        let wrapper = intercept::BeforeCommit {
            store: store(&db),
            callback: Box::new(move || {
                if revoke {
                    postgres::Client::connect(&url, postgres::NoTls)
                        .unwrap()
                        .execute(
                            "DELETE FROM case_memberships WHERE user_id=$1",
                            &[&actor.as_uuid()],
                        )
                        .unwrap();
                } else {
                    use application::procedural_resources::*;
                    procedural_resource_support::persist(
                        &resources,
                        case,
                        ResourceCommand {
                            operation_id: ResourceOperationId::new(),
                            resource_id: resource,
                            change: ResourceChange::Archive {
                                expected_revision: head,
                                reason: procedural_resource_support::text(
                                    "Archive while association is reviewed",
                                ),
                            },
                        },
                    );
                }
            }),
        };
        let workflow = service_with_store(&db, actor, Role::Litigator, Arc::new(wrapper));
        let before = associations(&mut db);
        let error = workflow
            .submit(
                "session",
                db.case,
                resource,
                command,
                draft.submission_digest,
            )
            .unwrap_err();
        assert!(matches!(
            (revoke, error),
            (true, ApplicationError::CaseNotFound)
                | (
                    false,
                    ApplicationError::ResourceActivity(
                        ResourceActivityError::ResourceRevisionConflict
                    )
                )
        ));
        assert_eq!(associations(&mut db), before);
        let committed: i64 = db
            .admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='resource_activity.link'",
                &[],
            )
            .unwrap()
            .get(0);
        assert_eq!(committed, 0);
    }
}
