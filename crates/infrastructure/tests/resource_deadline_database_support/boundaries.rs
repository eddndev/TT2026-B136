use super::*;
use crate::{
    deadline_backend_support as dl, procedural_fact_backend_support as facts,
    resource_activity_support as links,
};
use application::{
    cases::*, deadlines::*, procedural_facts::*, resource_activities::*, resource_deadlines::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    deadline_triggers::TriggerSourceRef,
    identity::{Role, UserId},
};
use std::sync::Arc;
struct BeforeCommit {
    store: Arc<dyn ResourceDeadlineStore>,
    callback: Box<dyn Fn() + Send + Sync>,
}
impl ResourceDeadlineStore for BeforeCommit {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlinePreparation, ApplicationError> {
        self.store.prepare(actor, case, resource, command)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceDeadline,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        (self.callback)();
        self.store.commit(actor, case, resource, prepared)
    }
}
#[test]
fn an_ordinary_deadline_and_link_cannot_forge_contextual_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let deadline = dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        command.deadline.clone(),
    );
    let mut link = captures.link();
    link.association_id = command.association_id;
    link.operation_id =
        ResourceActivityOperationId::from_uuid(deadline.receipt.operation_id.as_uuid());
    let ResourceActivityChange::Link { selection } = &mut link.change else {
        unreachable!()
    };
    selection.target = ResourceActivityTarget::Deadline {
        id: deadline.id,
        revision: deadline.revision,
        capture_digest: deadline.receipt.capture_digest,
    };
    links::persist(
        &links::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        link,
    );
    let before = atomic_rows(&mut db);
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
    assert_eq!(atomic_rows(&mut db), before);
}
#[test]
fn commit_revalidates_membership_and_live_source_after_read_preparation() {
    for revoke in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let (captures, command) = setup(&mut db);
        let actor = db.user("litigator", true);
        let draft = service(&db, actor, Role::Litigator)
            .prepare("session", db.case, captures.resource.id, command.clone())
            .unwrap();
        let source_command = command.deadline.clone().into_parts().0;
        let DeadlineChange::Register { definition } = source_command.change else {
            unreachable!()
        };
        let FactDeclaration::Known(TriggerSourceRef::Resolution(reference)) =
            definition.input.selection.source
        else {
            unreachable!()
        };
        let source_workflow = facts::service(&db, db.owner, Role::Owner);
        let source = source_workflow
            .get(
                "session",
                db.case,
                FactTarget::Resolution(reference.id),
                None,
            )
            .unwrap();
        let url = db.admin_url.clone();
        let case = db.case;
        let wrapper = BeforeCommit {
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
                    facts::persist(&source_workflow, case, facts::withdraw(&source));
                }
            }),
        };
        let workflow = service_with_store(&db, actor, Role::Litigator, Arc::new(wrapper));
        assert!(workflow
            .submit(
                "session",
                case,
                captures.resource.id,
                command,
                draft.submission_digest
            )
            .is_err());
        assert_eq!(
            pair_rows(&mut db),
            serde_json::json!({"deadlines":[],"associations":[]})
        );
        let count:i64=db.admin.query_one("SELECT count(*) FROM audit_events WHERE action IN ('deadline.registered','resource_activity.link','resource_deadline.registered')",&[]).unwrap().get(0);
        assert_eq!(count, 0);
    }
}
#[test]
fn exact_replay_survives_closure_but_new_creation_and_wrong_case_do_not() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap();
    let result = workflow
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    assert_eq!(
        workflow
            .submit(
                "session",
                db.case,
                captures.resource.id,
                command.clone(),
                draft.submission_digest
            )
            .unwrap(),
        result
    );
    assert!(workflow
        .prepare(
            "session",
            CaseId::new(),
            captures.resource.id,
            command.clone()
        )
        .is_err());
    let mut next = command;
    next.association_id = ResourceActivityId::new();
    let (mut deadline, policies) = next.deadline.into_parts();
    deadline.operation_id = DeadlineOperationId::new();
    deadline.deadline_id = DeadlineId::new();
    next.deadline = DeadlineHumanCommand::new(deadline, policies).unwrap();
    assert!(matches!(
        workflow.prepare("session", db.case, captures.resource.id, next),
        Err(ApplicationError::CaseClosed)
    ));
}
