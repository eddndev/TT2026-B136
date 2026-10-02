#[path = "resource_activity_target_support/access.rs"]
mod access;
mod case_administration_support;
mod case_stage_database_support;
#[path = "resource_activity_target_support/concurrency.rs"]
mod concurrency;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{cases::*, resource_activities::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use resource_activity_support::*;

fn query(
    limit: u32,
    after: Option<ResourceActivityId>,
    status: Option<ResourceActivityStatus>,
) -> ResourceActivityTargetQuery {
    ResourceActivityTargetQuery::new(limit, after, status).unwrap()
}
fn target(c: &Captures) -> ResourceActivityTargetId {
    ResourceActivityTargetId::Hearing(c.hearing.snapshot.id)
}

#[test]
fn existing_target_without_links_has_audited_observation_and_missing_scope_is_not_empty() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let adapter = store(&db);
    let before = independent_rows(&mut db);
    let result = adapter
        .list_for_target(
            db.owner,
            db.case,
            target(&captures),
            query(10, None, None),
            db.at,
        )
        .unwrap();
    assert!(result.associations.is_empty());
    assert_eq!(result.checked_at, db.at);
    assert!(!result.has_more);
    assert_eq!(result.next_after_id, None);
    assert_eq!(independent_rows(&mut db), before);
    let audit: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='resource_activity.target_list'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(audit, 1);
    assert!(matches!(
        adapter.list_for_target(
            db.owner,
            CaseId::new(),
            target(&captures),
            query(10, None, None),
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(adapter
        .list_for_target(
            db.owner,
            db.case,
            ResourceActivityTargetId::Hearing(application::hearings::HearingId::new()),
            query(10, None, None),
            db.at
        )
        .is_err());
    assert!(adapter
        .list_for_target(
            db.owner,
            db.case,
            ResourceActivityTargetId::Deadline(application::deadlines::DeadlineId::from_uuid(
                captures.hearing.snapshot.id.as_uuid()
            )),
            query(10, None, None),
            db.at
        )
        .is_err());
}

#[test]
fn inverse_pagination_filters_current_status_before_limit_across_resources() {
    use application::procedural_resources::*;
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let resources = procedural_resource_support::service(&db, db.owner, Role::Owner);
    let mut linked = Vec::new();
    for number in 1..=3 {
        let resource = if number == 1 {
            captures.resource.clone()
        } else {
            procedural_resource_support::persist(
                &resources,
                db.case,
                ResourceCommand {
                    operation_id: ResourceOperationId::new(),
                    resource_id: ResourceId::new(),
                    change: ResourceChange::Register {
                        values: captures.resource.values.clone(),
                    },
                },
            )
        };
        let mut command = captures.link();
        command.association_id = ResourceActivityId::from_uuid(uuid::Uuid::from_u128(number));
        if number != 1 {
            command.expected_resource_revision = resource.revision;
            let ResourceActivityChange::Link { selection } = &mut command.change else {
                unreachable!()
            };
            selection.resource = ResourceCaptureRef {
                id: resource.id,
                revision: resource.revision,
                capture_digest: resource.receipt.capture_digest,
            };
            selection.act = None;
        }
        linked.push(persist(&workflow, db.case, resource.id, command));
    }
    let removed = persist(
        &workflow,
        db.case,
        captures.resource.id,
        unlink(&linked[0], captures.head.revision),
    );
    let before = independent_rows(&mut db);
    let first = workflow
        .list_for_target(
            "session",
            db.case,
            target(&captures),
            query(1, None, Some(ResourceActivityStatus::Linked)),
        )
        .unwrap();
    assert_eq!(first.associations[0].association, linked[1]);
    assert_eq!(first.next_after_id, Some(linked[1].id));
    assert!(first.has_more);
    let second = workflow
        .list_for_target(
            "session",
            db.case,
            target(&captures),
            query(1, first.next_after_id, Some(ResourceActivityStatus::Linked)),
        )
        .unwrap();
    assert_eq!(second.associations[0].association, linked[2]);
    assert!(!second.has_more);
    assert_eq!(second.next_after_id, None);
    let all = workflow
        .list_for_target("session", db.case, target(&captures), query(10, None, None))
        .unwrap();
    assert_eq!(all.associations.len(), 3);
    assert_eq!(all.associations[0].association, removed);
    assert!(all
        .associations
        .iter()
        .all(|v| v.checked_at == all.checked_at));
    let unlinked = workflow
        .list_for_target(
            "session",
            db.case,
            target(&captures),
            query(10, None, Some(ResourceActivityStatus::Unlinked)),
        )
        .unwrap();
    assert_eq!(unlinked.associations.len(), 1);
    assert_eq!(unlinked.associations[0].association, removed);
    assert_eq!(independent_rows(&mut db), before);
    let own_case = db.case;
    hearing_database_support::complete(&mut db);
    let foreign = hearing_database_support::persist(
        &hearing_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        hearing_database_support::schedule(),
    );
    assert!(workflow
        .list_for_target(
            "session",
            own_case,
            ResourceActivityTargetId::Hearing(foreign.snapshot.id),
            query(10, None, None)
        )
        .is_err());
}

#[test]
fn historical_deadline_link_keeps_current_retired_projection_without_fallback_due() {
    use deadline_backend_support as deadlines;
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let deadline_service = deadlines::service(&db, db.owner, Role::Owner);
    let original = deadlines::persist(
        &deadline_service,
        db.case,
        deadlines::human(deadlines::setup(&db), Some(deadlines::FOLLOW_RESOLUTION)),
    );
    let mut command = captures.link();
    let ResourceActivityChange::Link { selection } = &mut command.change else {
        unreachable!()
    };
    selection.target = ResourceActivityTarget::Deadline {
        id: original.id,
        revision: original.revision,
        capture_digest: original.receipt.capture_digest,
    };
    let workflow = service(&db, db.owner, Role::Owner);
    let linked = persist(&workflow, db.case, captures.resource.id, command);
    let retired = deadlines::persist(
        &deadline_service,
        db.case,
        deadlines::human(deadlines::retire(&original), None),
    );
    captures.archive(&db);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let before = independent_rows(&mut db);
    let page = workflow
        .list_for_target(
            "session",
            db.case,
            ResourceActivityTargetId::Deadline(original.id),
            query(10, None, None),
        )
        .unwrap();
    assert_eq!(page.associations.len(), 1);
    let view = &page.associations[0];
    assert_eq!(view.association, linked);
    assert_eq!(
        view.association.sources.target,
        ResourceActivityTargetDetail::Deadline(Box::new(original))
    );
    let ResourceActivityCurrentTarget::Deadline(current) = &view.current_target else {
        unreachable!()
    };
    assert_eq!(current.detail(), &retired);
    assert_eq!(current.operational().due_at(), None);
    assert_eq!(independent_rows(&mut db), before);
}
