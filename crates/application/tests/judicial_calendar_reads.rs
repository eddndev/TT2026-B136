#[allow(dead_code)]
mod case_support;
mod judicial_calendar_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::{crypto::Sha256Digest, identity::Role};
use judicial_calendar_support::*;

fn query() -> JudicialCalendarQuery {
    JudicialCalendarQuery::new(
        20,
        None,
        JudicialCalendarStatusFilter::Published,
        None,
        None,
    )
    .unwrap()
}
#[test]
fn staff_can_query_exact_retired_revision_and_days_outside_coverage() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (identity, actor) = identity(role, 1);
        let first = detail(actor.id, &command(), values("Calendar"));
        let result = detail(actor.id, &retirement(&first), first.values.clone());
        let id = result.id;
        let revision = result.revision;
        let digest = result.values_digest;
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, requested, rev, at| {
                assert_eq!(requested, id);
                assert_eq!(rev, Some(revision));
                assert_eq!(at, instant());
                Ok(result)
            });
        let (service, clock) = service(store, identity);
        let end = "9999-12-31".parse().unwrap();
        let days = service
            .days(
                "session",
                id,
                revision,
                JudicialCalendarDaysQuery::new(end, end).unwrap(),
            )
            .unwrap();
        assert_eq!(days.values_digest, digest);
        assert_eq!(days.days.len(), 1);
        assert_eq!(days.days[0].date(), end);
        assert!(days.days[0].classification().is_none());
        assert_eq!(clock.calls(), 1);
    }
}
#[test]
fn get_rejects_another_root_or_revision_even_with_valid_receipt() {
    for revision_mismatch in [false, true] {
        let (identity, actor) = identity(Role::Paralegal, 1);
        let d = detail(actor.id, &command(), values("Calendar"));
        let id = if revision_mismatch {
            d.id
        } else {
            JudicialCalendarId::new()
        };
        let revision = if revision_mismatch {
            JudicialCalendarRevision::new(2).unwrap()
        } else {
            d.revision
        };
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _| Ok(d));
        let (service, _) = service(store, identity);
        assert!(matches!(
            service.get("session", id, Some(revision)),
            Err(ApplicationError::JudicialCalendar(
                JudicialCalendarError::StoredInconsistent(_)
            ))
        ));
    }
}
#[test]
fn list_accepts_nil_id_and_rejects_bad_order_cursor_filter_and_length() {
    for variant in 0..6 {
        let (identity, actor) = identity(Role::Litigator, 1);
        let mut cmd = command();
        cmd.calendar_id = JudicialCalendarId::from_uuid(uuid::Uuid::nil());
        let detail = detail(actor.id, &cmd, values("Calendar"));
        let entry = JudicialCalendarOverview::from(&detail);
        let mut page = JudicialCalendarPage {
            calendars: vec![entry.clone()],
            has_more: false,
            next_after_id: None,
        };
        match variant {
            1 => page.next_after_id = Some(entry.id),
            2 => {
                page.has_more = true;
                page.next_after_id = Some(entry.id);
            }
            3 => page.calendars.push(entry.clone()),
            4 => page.calendars[0].status = JudicialCalendarStatus::Retired,
            5 => page.calendars = vec![entry; 21],
            _ => {}
        }
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _| Ok(page));
        let (service, _) = service(store, identity);
        let result = service.list("session", query());
        if variant == 0 {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::JudicialCalendar(
                    JudicialCalendarError::StoredInconsistent(_)
                ))
            ));
        }
    }
}
#[test]
fn history_validates_scope_receipts_descending_sequence_and_cursor() {
    for variant in 0..5 {
        let (identity, actor) = identity(Role::Owner, 1);
        let first = detail(actor.id, &command(), values("Calendar"));
        let second = detail(actor.id, &replacement(&first), first.values.clone());
        let id = first.id;
        let mut page = JudicialCalendarHistoryPage {
            revisions: vec![
                JudicialCalendarHistoryEntry::from(&second),
                JudicialCalendarHistoryEntry::from(&first),
            ],
            has_more: false,
            next_before_revision: None,
        };
        match variant {
            1 => page.revisions.reverse(),
            2 => page.revisions[0].id = JudicialCalendarId::new(),
            3 => page.revisions[0].receipt.submission_digest = Sha256Digest::from_array([0; 32]),
            4 => page.next_before_revision = Some(JudicialCalendarRevision::initial()),
            _ => {}
        }
        let mut store = MockStore::new();
        store
            .expect_history()
            .times(1)
            .return_once(move |_, _, _, _| Ok(page));
        let (service, _) = service(store, identity);
        let result = service.history(
            "session",
            id,
            JudicialCalendarHistoryQuery::new(10, None).unwrap(),
        );
        assert_eq!(result.is_ok(), variant == 0);
    }
}
#[test]
fn entity_query_is_explicit_and_does_not_trim_or_coerce() {
    for code in ["00", "33", "9", " 09", "09 ", "Mexico"] {
        assert!(JudicialCalendarQuery::new(
            1,
            None,
            JudicialCalendarStatusFilter::All,
            None,
            Some(code)
        )
        .is_err());
    }
    for code in ["01", "09", "32"] {
        let q = JudicialCalendarQuery::new(
            1,
            None,
            JudicialCalendarStatusFilter::All,
            None,
            Some(code),
        )
        .unwrap();
        assert_eq!(q.entity_code(), Some(code));
    }
}
