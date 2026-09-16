mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;

use application::{hearings::*, ApplicationError};
use domain::identity::Role;
use hearing_database_support::*;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn range(
    limit: u32,
    after: Option<HearingAgendaCursor>,
    status: HearingStatusFilter,
) -> HearingAgendaQuery {
    HearingAgendaQuery::new(
        limit,
        OffsetDateTime::parse("2026-09-01T00:00:00Z", &Rfc3339).unwrap(),
        OffsetDateTime::parse("2026-10-01T00:00:00Z", &Rfc3339).unwrap(),
        after,
        status,
    )
    .unwrap()
}
fn replace(
    service: &HearingService,
    case: domain::cases::CaseId,
    base: &HearingDetail,
    at: &str,
) -> HearingDetail {
    persist(
        service,
        case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: base.snapshot.id,
            change: HearingChange::Replace {
                expected_revision: base.snapshot.revision,
                context: context(),
                values: values(at),
                reason: HearingNote::new("Rescheduled").unwrap(),
            },
        },
    )
}
#[test]
fn list_and_agenda_filter_current_heads_before_pagination() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, db.case, schedule());
    let second = persist(&svc, db.case, schedule());
    let moved = persist(&svc, db.case, schedule());
    replace(&svc, db.case, &moved, "2026-11-15T09:00:00-06:00");
    let cancelled = persist(&svc, db.case, schedule());
    persist(
        &svc,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: cancelled.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: cancelled.snapshot.revision,
                reason: HearingNote::new("Cancelled").unwrap(),
            },
        },
    );
    let page = svc
        .list(
            "session",
            db.case,
            HearingQuery::new(100, None, HearingStatusFilter::Scheduled).unwrap(),
        )
        .unwrap();
    assert_eq!(page.hearings.len(), 3);
    assert!(!page.hearings.iter().any(|h| h.id == cancelled.snapshot.id));
    assert_eq!(
        page.hearings
            .iter()
            .find(|h| h.id == moved.snapshot.id)
            .unwrap()
            .revision
            .get(),
        2
    );
    let first_page = svc
        .agenda("session", range(1, None, HearingStatusFilter::Scheduled))
        .unwrap();
    let second_page = svc
        .agenda(
            "session",
            range(1, first_page.next_after, HearingStatusFilter::Scheduled),
        )
        .unwrap();
    let mut ids = [first.snapshot.id, second.snapshot.id];
    ids.sort_by_key(|id| id.as_uuid());
    assert_eq!(first_page.hearings[0].id, ids[0]);
    assert!(first_page.has_more);
    assert_eq!(second_page.hearings[0].id, ids[1]);
    assert!(!second_page.has_more);
    let cancelled_page = svc
        .agenda("session", range(100, None, HearingStatusFilter::Cancelled))
        .unwrap();
    assert_eq!(cancelled_page.hearings.len(), 1);
    assert_eq!(cancelled_page.hearings[0].id, cancelled.snapshot.id);
    assert_eq!(cancelled_page.hearings[0].revision.get(), 2);
    let mut all = Vec::new();
    let mut cursor = None;
    loop {
        let page = svc
            .list(
                "session",
                db.case,
                HearingQuery::new(1, cursor, HearingStatusFilter::All).unwrap(),
            )
            .unwrap();
        all.extend(page.hearings.into_iter().map(|h| h.id));
        if !page.has_more {
            break;
        }
        cursor = page.next_after_id;
    }
    assert_eq!(all.len(), 4);
    assert!(all.windows(2).all(|v| v[0].as_uuid() < v[1].as_uuid()));
}
#[test]
fn agenda_scopes_membership_before_limit_and_reloads_role() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let owner = service(&db, db.owner, Role::Owner);
    let ours = persist(&owner, db.case, schedule());
    let member = db.user("litigator", true);
    let old_case = db.case;
    complete(&mut db);
    persist(&owner, db.case, schedule());
    let member_service = service(&db, member, Role::Litigator);
    let page = member_service
        .agenda("session", range(1, None, HearingStatusFilter::All))
        .unwrap();
    assert_eq!(page.hearings.len(), 1);
    assert_eq!(page.hearings[0].id, ours.snapshot.id);
    assert!(!page.has_more);
    assert!(matches!(
        member_service.list(
            "session",
            db.case,
            HearingQuery::new(100, None, HearingStatusFilter::All).unwrap()
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&old_case.as_uuid(), &member.as_uuid()],
        )
        .unwrap();
    assert!(member_service
        .agenda("session", range(100, None, HearingStatusFilter::All))
        .unwrap()
        .hearings
        .is_empty());
    db.admin
        .execute(
            "UPDATE users SET role='client' WHERE id=$1",
            &[&member.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        member_service.agenda("session", range(100, None, HearingStatusFilter::All)),
        Err(ApplicationError::PermissionDenied)
    ));
}
