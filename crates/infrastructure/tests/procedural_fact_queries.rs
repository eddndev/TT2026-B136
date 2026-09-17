mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
use application::{cases::*, procedural_facts::*};
use domain::identity::Role;
use procedural_fact_backend_support::*;
use uuid::Uuid;

#[test]
fn resolution_pages_filter_current_heads_before_limit_and_preserve_nil_cursor() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let mut saved = Vec::new();
    for index in 0..3 {
        saved.push(persist(
            &workflow,
            db.case,
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                FactOperationId::new(),
                ResolutionId::from_uuid(Uuid::from_u128(index)),
                FactChange::record(values("Declared record")),
            )),
        ));
    }
    let corrected = persist(&workflow, db.case, correct(&saved[1]));
    let withdrawn = persist(&workflow, db.case, withdraw(&corrected));
    let first = workflow
        .list_resolutions(
            "session",
            db.case,
            ResolutionQuery::new(1, None, FactStatusFilter::Recorded).unwrap(),
        )
        .unwrap();
    assert_eq!(first.resolutions.len(), 1);
    assert_eq!(first.resolutions[0].root.id().as_uuid(), Uuid::nil());
    assert!(first.has_more);
    let second = workflow
        .list_resolutions(
            "session",
            db.case,
            ResolutionQuery::new(1, first.next_after_id, FactStatusFilter::Recorded).unwrap(),
        )
        .unwrap();
    assert_eq!(second.resolutions.len(), 1);
    assert_eq!(
        second.resolutions[0].root.id().as_uuid(),
        Uuid::from_u128(2)
    );
    assert!(!second.has_more);
    assert_eq!(second.next_after_id, None);
    let retired = workflow
        .list_resolutions(
            "session",
            db.case,
            ResolutionQuery::new(1, None, FactStatusFilter::Withdrawn).unwrap(),
        )
        .unwrap();
    assert_eq!(
        retired.resolutions[0].revision,
        withdrawn.snapshot.metadata().revision
    );
    assert!(!retired.has_more);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let history = workflow
        .history(
            "session",
            db.case,
            withdrawn.snapshot.target(),
            FactHistoryQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        history
            .revisions
            .iter()
            .map(|r| r.metadata.revision.get())
            .collect::<Vec<_>>(),
        vec![3, 2]
    );
    assert!(history.has_more);
    let last = workflow
        .history(
            "session",
            db.case,
            withdrawn.snapshot.target(),
            FactHistoryQuery::new(2, history.next_before_revision.map(|r| r.get())).unwrap(),
        )
        .unwrap();
    assert_eq!(last.revisions.len(), 1);
    assert_eq!(last.revisions[0].metadata.revision.get(), 1);
    assert!(!last.has_more);
    assert_eq!(last.next_before_revision, None);
}

#[test]
fn notification_pages_keep_the_exact_parent_and_exclude_retired_heads() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let parent = persist(&workflow, db.case, record());
    let other = persist(&workflow, db.case, record());
    let reference = resolution_ref(&parent);
    let mut notices = Vec::new();
    for index in 0..3 {
        notices.push(persist(
            &workflow,
            db.case,
            ProceduralFactCommand::Notification(
                NotificationCommand::new(
                    FactOperationId::new(),
                    NotificationId::from_uuid(Uuid::from_u128(index)),
                    reference.id,
                    FactChange::record(notification_values(reference, "Declared practice")),
                )
                .unwrap(),
            ),
        ));
    }
    persist(&workflow, db.case, withdraw(&notices[1]));
    persist(&workflow, db.case, notify(resolution_ref(&other)));
    let first = workflow
        .list_notifications(
            "session",
            db.case,
            reference.id,
            NotificationQuery::new(1, None, FactStatusFilter::Recorded).unwrap(),
        )
        .unwrap();
    assert_eq!(first.notifications[0].root.id().as_uuid(), Uuid::nil());
    assert!(first.has_more);
    let second = workflow
        .list_notifications(
            "session",
            db.case,
            reference.id,
            NotificationQuery::new(1, first.next_after_id, FactStatusFilter::Recorded).unwrap(),
        )
        .unwrap();
    assert_eq!(second.notifications.len(), 1);
    assert_eq!(
        second.notifications[0].root.id().as_uuid(),
        Uuid::from_u128(2)
    );
    assert_eq!(second.notifications[0].resolution, reference);
    assert!(!second.has_more);
    assert!(workflow
        .list_notifications(
            "session",
            db.case,
            ResolutionId::new(),
            NotificationQuery::new(1, None, FactStatusFilter::All).unwrap()
        )
        .is_err());
}

#[test]
fn audit_failure_prevents_detail_history_and_list_responses() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let parent = persist(&workflow, db.case, record());
    let reference = resolution_ref(&parent);
    persist(&workflow, db.case, notify(reference));
    db.admin.batch_execute("CREATE FUNCTION reject_fact_read_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit error'; END; $$; CREATE TRIGGER reject_fact_read_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_fact_read_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(workflow
        .get("session", db.case, parent.snapshot.target(), None)
        .is_err());
    assert!(workflow
        .history(
            "session",
            db.case,
            parent.snapshot.target(),
            FactHistoryQuery::new(1, None).unwrap()
        )
        .is_err());
    assert!(workflow
        .list_resolutions(
            "session",
            db.case,
            ResolutionQuery::new(1, None, FactStatusFilter::All).unwrap()
        )
        .is_err());
    assert!(workflow
        .list_notifications(
            "session",
            db.case,
            reference.id,
            NotificationQuery::new(1, None, FactStatusFilter::All).unwrap()
        )
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}
