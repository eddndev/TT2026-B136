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
use application::{hearings::*, resource_activities::*};
use domain::identity::Role;
use resource_activity_support::*;

#[test]
fn historical_resource_act_and_hearing_survive_current_changes_and_unlink() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let hearing = hearing_database_support::persist(
        &hearing_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: captures.hearing.snapshot.id,
            change: HearingChange::Replace {
                expected_revision: captures.hearing.snapshot.revision,
                context: hearing_database_support::context(),
                values: hearing_database_support::values("2026-10-01T09:00:00-06:00"),
                reason: HearingNote::new("Correct hearing date").unwrap(),
            },
        },
    );
    let workflow = service(&db, db.owner, Role::Owner);
    let independent = independent_rows(&mut db);
    let linked = persist(&workflow, db.case, captures.resource.id, captures.link());
    assert_eq!(independent_rows(&mut db), independent);
    assert_eq!(linked.sources.resource, captures.resource);
    assert_eq!(linked.sources.act, Some(captures.act.clone()));
    assert_eq!(
        linked.recorded_resource_head.revision,
        captures.head.revision
    );
    assert_eq!(
        linked.sources.target,
        ResourceActivityTargetDetail::Hearing(Box::new(captures.hearing.clone()))
    );
    let view = workflow
        .get("session", db.case, captures.resource.id, linked.id, None)
        .unwrap();
    assert_eq!(view.association, linked);
    assert_eq!(
        view.current_target,
        ResourceActivityCurrentTarget::Hearing(Box::new(hearing))
    );
    assert_eq!(view.checked_at, db.at);
    let archived = captures.archive(&db);
    let independent = independent_rows(&mut db);
    let unlinked = persist(
        &workflow,
        db.case,
        captures.resource.id,
        unlink(&linked, archived.revision),
    );
    assert_eq!(independent_rows(&mut db), independent);
    assert_eq!(unlinked.sources, linked.sources);
    assert_eq!(unlinked.selection, linked.selection);
    assert_eq!(unlinked.status, ResourceActivityStatus::Unlinked);
    assert_eq!(
        unlinked.receipt.previous.unwrap().capture_digest,
        linked.receipt.capture_digest
    );
    assert_eq!(
        workflow
            .history(
                "session",
                db.case,
                captures.resource.id,
                linked.id,
                history_query()
            )
            .unwrap()
            .revisions,
        vec![unlinked.clone(), linked.clone()]
    );
    assert_eq!(
        workflow
            .get(
                "session",
                db.case,
                captures.resource.id,
                linked.id,
                Some(linked.revision)
            )
            .unwrap()
            .association,
        linked
    );
    assert!(workflow
        .list(
            "session",
            db.case,
            captures.resource.id,
            query(None, Some(ResourceActivityStatus::Linked))
        )
        .unwrap()
        .associations
        .is_empty());
    assert_eq!(
        workflow
            .list(
                "session",
                db.case,
                captures.resource.id,
                query(
                    Some(ResourceActivityKind::Hearing),
                    Some(ResourceActivityStatus::Unlinked)
                )
            )
            .unwrap()
            .associations[0]
            .association,
        unlinked
    );
}

#[test]
fn deadline_historical_capture_never_supplies_operational_due_for_an_old_head() {
    use deadline_backend_support as deadlines;
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let deadline_service = deadlines::service(&db, db.owner, Role::Owner);
    let original = deadlines::persist(
        &deadline_service,
        db.case,
        deadlines::human(deadlines::setup(&db), Some(deadlines::FOLLOW_RESOLUTION)),
    );
    assert!(original.calculation.result.due_at().is_some());
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
    let independent = independent_rows(&mut db);
    let linked = persist(&workflow, db.case, captures.resource.id, command);
    assert_eq!(independent_rows(&mut db), independent);
    let corrected = deadlines::persist(
        &deadline_service,
        db.case,
        deadlines::human(
            deadlines::correct(&original),
            Some(deadlines::FOLLOW_RESOLUTION),
        ),
    );
    let view = workflow
        .get("session", db.case, captures.resource.id, linked.id, None)
        .unwrap();
    assert_eq!(
        view.association.sources.target,
        ResourceActivityTargetDetail::Deadline(Box::new(original.clone()))
    );
    let ResourceActivityCurrentTarget::Deadline(current) = view.current_target else {
        unreachable!()
    };
    assert_eq!(current.detail(), &corrected);
    assert_eq!(
        current.operational().due_at(),
        corrected.calculation.result.due_at()
    );
    assert_eq!(current.operational().checked_at(), Some(view.checked_at));
    let retired = deadlines::persist(
        &deadline_service,
        db.case,
        deadlines::human(deadlines::retire(&corrected), None),
    );
    let view = workflow
        .list(
            "session",
            db.case,
            captures.resource.id,
            query(Some(ResourceActivityKind::Deadline), None),
        )
        .unwrap()
        .associations
        .remove(0);
    let ResourceActivityCurrentTarget::Deadline(current) = view.current_target else {
        unreachable!()
    };
    assert_eq!(current.detail(), &retired);
    assert_eq!(current.operational().due_at(), None);
    assert_eq!(view.association, linked);
}
