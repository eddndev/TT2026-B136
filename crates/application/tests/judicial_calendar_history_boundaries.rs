#[allow(dead_code)]
mod case_support;
mod judicial_calendar_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::Role;
use judicial_calendar_support::*;

#[test]
fn reversed_history_with_maximum_revision_returns_error_without_overflow() {
    let (identity, actor) = identity(Role::Owner, 1);
    let first = detail(actor.id, &command(), values("Calendar"));
    let mut cmd = replacement(&first);
    if let JudicialCalendarChange::Replace {
        expected_revision, ..
    } = &mut cmd.change
    {
        *expected_revision = JudicialCalendarRevision::new(u32::MAX - 1).unwrap();
    }
    let highest = detail(actor.id, &cmd, first.values.clone());
    let id = first.id;
    let page = JudicialCalendarHistoryPage {
        revisions: vec![
            JudicialCalendarHistoryEntry::from(&first),
            JudicialCalendarHistoryEntry::from(&highest),
        ],
        has_more: false,
        next_before_revision: None,
    };
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, _| Ok(page));
    let (service, _) = service(store, identity);
    assert!(matches!(
        service.history(
            "session",
            id,
            JudicialCalendarHistoryQuery::new(10, None).unwrap()
        ),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::StoredInconsistent(_)
        ))
    ));
}
#[test]
fn nonadjacent_duplicate_operation_in_history_is_inconsistent() {
    let (identity, actor) = identity(Role::Owner, 1);
    let first = detail(actor.id, &command(), values("Calendar"));
    let second = detail(actor.id, &replacement(&first), first.values.clone());
    let mut third_command = replacement(&second);
    third_command.operation_id = first.receipt.operation_id;
    let third = detail(actor.id, &third_command, first.values.clone());
    let id = first.id;
    let page = JudicialCalendarHistoryPage {
        revisions: vec![
            JudicialCalendarHistoryEntry::from(&third),
            JudicialCalendarHistoryEntry::from(&second),
            JudicialCalendarHistoryEntry::from(&first),
        ],
        has_more: false,
        next_before_revision: None,
    };
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, _| Ok(page));
    let (service, _) = service(store, identity);
    assert!(matches!(
        service.history(
            "session",
            id,
            JudicialCalendarHistoryQuery::new(10, None).unwrap()
        ),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn an_empty_history_requires_the_exclusive_first_revision_boundary() {
    for before in [None, Some(1), Some(2)] {
        let (identity, _) = identity(Role::Owner, 1);
        let mut store = MockStore::new();
        store
            .expect_history()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(JudicialCalendarHistoryPage {
                    revisions: vec![],
                    has_more: false,
                    next_before_revision: None,
                })
            });
        let (service, _) = service(store, identity);
        let result = service.history(
            "session",
            JudicialCalendarId::new(),
            JudicialCalendarHistoryQuery::new(10, before).unwrap(),
        );
        assert_eq!(result.is_ok(), before == Some(1));
    }
}
