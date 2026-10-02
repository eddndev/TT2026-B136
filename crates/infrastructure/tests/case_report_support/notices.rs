use crate::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use time::Duration;

#[test]
fn own_ready_notice_is_durable_unread_until_explicit_ack_and_ack_replay_keeps_first_instant() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Notice case", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let store = store(&db, timer.clone());
    let ready = finish(&store, &actor, command(at), at);
    let notice = ready.notice.as_ref().unwrap();
    assert_eq!(notice.kind, CaseReportNoticeKind::Ready);
    assert!(notice.read_at.is_none());
    store
        .download(&actor, ready.id, CaseReportFormat::Csv, at)
        .unwrap();
    assert!(store
        .get(&actor, ready.id, at)
        .unwrap()
        .notice
        .unwrap()
        .read_at
        .is_none());
    let other_id = db.user("owner", false);
    let other = principal(&mut db, other_id);
    assert!(matches!(
        store.acknowledge_notice(&other, ready.id, at),
        Err(ApplicationError::CaseReport(CaseReportError::NotFound))
    ));
    let read_at = at + Duration::seconds(5);
    timer.set(read_at);
    let read = store.acknowledge_notice(&actor, ready.id, read_at).unwrap();
    assert_eq!(read.notice.as_ref().unwrap().read_at, Some(read_at));
    let later = at + Duration::seconds(10);
    timer.set(later);
    assert_eq!(
        store.acknowledge_notice(&actor, ready.id, later).unwrap(),
        read
    );
    assert_eq!(count(&mut db, "case_report_notices"), 1);
    assert_eq!(audits(&mut db, "case_report.notice_read"), 1);
}

#[test]
fn unread_filter_applies_before_limit_and_never_exposes_another_requester_notice() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Unread selection", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    let mut ready = Vec::new();
    for _ in 0..3 {
        ready.push(finish(&store, &actor, command(at), at));
    }
    ready.sort_by_key(|report| report.id.as_uuid());
    store.acknowledge_notice(&actor, ready[0].id, at).unwrap();
    let page = store
        .list(
            &actor,
            CaseReportQuery {
                limit: 1,
                after_id: None,
                unread_only: true,
            },
            at,
        )
        .unwrap();
    assert_eq!(page.reports.len(), 1);
    assert_eq!(page.reports[0].id, ready[1].id);
    assert!(page.has_more);
    assert_eq!(page.next_after_id, Some(ready[1].id));
    let second = store
        .list(
            &actor,
            CaseReportQuery {
                limit: 1,
                after_id: page.next_after_id,
                unread_only: true,
            },
            at,
        )
        .unwrap();
    assert_eq!(second.reports[0].id, ready[2].id);
    assert!(!second.has_more);
    assert!(second.next_after_id.is_none());
    let other_id = db.user("owner", false);
    let other = principal(&mut db, other_id);
    assert!(store
        .list(
            &other,
            CaseReportQuery {
                limit: 1,
                after_id: None,
                unread_only: true
            },
            at
        )
        .unwrap()
        .reports
        .is_empty());
}
