mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::Role;
use judicial_calendar_database_support::*;
use uuid::Uuid;

#[test]
fn calendar_lists_paginate_heads_and_filters_without_resurrecting_retired_rows() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let mut first = publish();
    first.calendar_id = JudicialCalendarId::from_uuid(Uuid::nil());
    let first = persist(&workflow, first);
    let mut second = publish();
    second.calendar_id = JudicialCalendarId::from_uuid(Uuid::from_u128(1));
    let second = persist(&workflow, second);
    let mut third = publish();
    third.calendar_id = JudicialCalendarId::from_uuid(Uuid::from_u128(2));
    let third = persist(&workflow, third);
    persist(&workflow, retire(&second));
    let page = workflow
        .list(
            "session",
            JudicialCalendarQuery::new(
                1,
                None,
                JudicialCalendarStatusFilter::Published,
                Some(JudicialCalendarJurisdiction::Federal),
                Some("09"),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        page.calendars.iter().map(|v| v.id).collect::<Vec<_>>(),
        vec![first.id]
    );
    assert!(page.has_more);
    assert_eq!(page.next_after_id, Some(first.id));
    let next = workflow
        .list(
            "session",
            JudicialCalendarQuery::new(
                1,
                page.next_after_id,
                JudicialCalendarStatusFilter::Published,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        next.calendars.iter().map(|v| v.id).collect::<Vec<_>>(),
        vec![third.id]
    );
    assert!(!next.has_more);
    assert_eq!(next.next_after_id, None);
    let retired = workflow
        .list(
            "session",
            JudicialCalendarQuery::new(10, None, JudicialCalendarStatusFilter::Retired, None, None)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(retired.calendars[0].id, second.id);
    assert_eq!(retired.calendars[0].revision.get(), 2);
    for (jurisdiction, entity) in [
        (Some(JudicialCalendarJurisdiction::Local), None),
        (None, Some("32")),
    ] {
        assert!(workflow
            .list(
                "session",
                JudicialCalendarQuery::new(
                    20,
                    None,
                    JudicialCalendarStatusFilter::All,
                    jurisdiction,
                    entity
                )
                .unwrap()
            )
            .unwrap()
            .calendars
            .is_empty());
    }
}
#[test]
fn history_uses_exclusive_descending_cursor_and_missing_exact_revision_does_not_fallback() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, publish());
    let second = persist(&workflow, replace(&first));
    let third = persist(&workflow, retire(&second));
    let first_page = workflow
        .history(
            "session",
            first.id,
            JudicialCalendarHistoryQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        first_page.revisions,
        vec![(&third).into(), (&second).into()]
    );
    assert!(first_page.has_more);
    assert_eq!(first_page.next_before_revision, Some(second.revision));
    let last = workflow
        .history(
            "session",
            first.id,
            JudicialCalendarHistoryQuery::new(2, Some(2)).unwrap(),
        )
        .unwrap();
    assert_eq!(last.revisions, vec![(&first).into()]);
    assert!(!last.has_more);
    let before = snapshot(&mut db);
    for (id, revision) in [
        (first.id, Some(JudicialCalendarRevision::new(4).unwrap())),
        (JudicialCalendarId::new(), None),
    ] {
        assert!(matches!(
            workflow.get("session", id, revision),
            Err(ApplicationError::JudicialCalendar(
                JudicialCalendarError::NotFound
            ))
        ));
    }
    assert_eq!(snapshot(&mut db), before);
}
