mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::Role;
use judicial_calendar_database_support::*;

#[test]
fn calendar_preparation_reserves_nothing_and_exact_history_keeps_retired_values() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = publish();
    let before = counts(&mut db);
    let draft = workflow.prepare("session", command.clone()).unwrap();
    assert_eq!(counts(&mut db), before);
    let first = workflow
        .submit("session", command.clone(), draft.submission_digest)
        .unwrap();
    assert_eq!(first.receipt.operation_id, command.operation_id);
    assert_eq!(first.receipt.submission_digest, draft.submission_digest);
    assert_eq!(first.recorded_by.id, db.owner);
    assert_eq!(first.recorded_by.email, "owner@example.test");
    let second = persist(&workflow, replace(&first));
    let third = persist(&workflow, retire(&second));
    assert_eq!(third.status, JudicialCalendarStatus::Retired);
    assert_eq!(third.values, second.values);
    assert_eq!(third.values_digest, second.values_digest);
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 3, before.2 + 3));
    assert_eq!(
        workflow
            .get("session", first.id, Some(first.revision))
            .unwrap(),
        first
    );
    assert_eq!(workflow.get("session", first.id, None).unwrap(), third);
    let history = workflow
        .history(
            "session",
            first.id,
            JudicialCalendarHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        history.revisions,
        vec![(&third).into(), (&second).into(), (&first).into()]
    );
    assert!(!history.has_more);
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare("session", replace(&third)),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::Retired
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn global_staff_reads_require_no_case_membership_but_client_and_nonowners_cannot_write() {
    let Some(mut db) = Fixture::new() else { return };
    let record = persist(&service(&db, db.owner, Role::Owner), publish());
    for (text, role) in [
        ("litigator", Role::Litigator),
        ("paralegal", Role::Paralegal),
    ] {
        let actor = db.user(text, false);
        let adapter = store(&db);
        assert_eq!(adapter.get(actor, record.id, None, db.at).unwrap(), record);
        let before = snapshot(&mut db);
        assert!(matches!(
            adapter.prepare(actor, &publish()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(snapshot(&mut db), before);
        let listed = service(&db, actor, role)
            .list(
                "session",
                JudicialCalendarQuery::new(
                    20,
                    None,
                    JudicialCalendarStatusFilter::Published,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(listed.calendars.len(), 1);
    }
    let client = db.user("client", true);
    let before = snapshot(&mut db);
    assert!(matches!(
        store(&db).get(client, record.id, None, db.at),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store(&db).history(
            client,
            record.id,
            JudicialCalendarHistoryQuery::new(10, None).unwrap(),
            db.at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn calendar_scope_is_fixed_and_operations_are_unique_across_roots() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let record = persist(&workflow, publish());
    let mut reused = publish();
    reused.operation_id = record.receipt.operation_id;
    let changed = JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: record.id,
        change: JudicialCalendarChange::Replace {
            expected_revision: record.revision,
            values: values("Another scope", "Review source"),
            reason: JudicialCalendarReason::new("Change scope").unwrap(),
        },
    };
    let before = snapshot(&mut db);
    assert!(matches!(
        store(&db).prepare(db.owner, &changed),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::ScopeChangeForbidden
        ))
    ));
    assert!(matches!(
        workflow.prepare("session", reused),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::OperationConflict
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
