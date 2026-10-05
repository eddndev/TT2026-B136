use super::*;
use application::ApplicationError;
use uuid::Uuid;

#[test]
fn precautionary_current_heads_filter_replacement_and_cancellation_before_pagination() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, mut command) = crate::hearing_fixture::setup(&mut db);
    let at = scheduled(&command);
    command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(1));
    let first = crate::hearing_fixture::persist(&db, actor.clone(), command.clone());
    command.operation_id = PrecautionaryHearingOperationId::new();
    command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(2));
    let second = crate::hearing_fixture::persist(&db, actor.clone(), command);
    let store = store(&db);
    let q = query(
        at,
        1,
        AgendaKind::PrecautionaryHearing,
        HearingStatusFilter::Scheduled,
        None,
    );
    let first_page = store.list(db.owner, q).unwrap();
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(
        appointment(&first_page.items[0]).id,
        first.capture.review.command.hearing_id
    );
    assert!(!first_page.complete);
    let cursor = first_page.next_after.unwrap();
    assert_eq!(cursor.kind(), AgendaItemKind::PrecautionaryHearing);
    assert_eq!(cursor.id(), Uuid::from_u128(1));
    let second_page = store
        .list(
            db.owner,
            query(
                at,
                1,
                AgendaKind::PrecautionaryHearing,
                HearingStatusFilter::Scheduled,
                Some(cursor),
            ),
        )
        .unwrap();
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(
        appointment(&second_page.items[0]).id,
        second.capture.review.command.hearing_id
    );
    assert!(second_page.complete && second_page.next_after.is_none());

    let later = at + Duration::days(1);
    let mut replacement = crate::hearing_fixture::replacement(&first);
    let PrecautionaryHearingChange::Replace { values, .. } = &mut replacement.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.scheduled_at = HearingTime::new(later).unwrap();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let replaced = crate::hearing_fixture::persist(&db, actor.clone(), replacement);
    let remaining = store.list(db.owner, q).unwrap();
    assert_eq!(remaining.items.len(), 1);
    assert_eq!(
        appointment(&remaining.items[0]).id,
        second.capture.review.command.hearing_id
    );
    assert!(remaining.complete && remaining.next_after.is_none());
    let later_query = query(
        later,
        1,
        AgendaKind::PrecautionaryHearing,
        HearingStatusFilter::Scheduled,
        None,
    );
    let page = store.list(db.owner, later_query).unwrap();
    assert_eq!(page.items.len(), 1);
    let shown = appointment(&page.items[0]);
    assert_eq!(shown.revision, replaced.capture.review.result_revision);
    assert_eq!(shown.scheduled_at.utc(), later);
    assert_eq!(shown.capture_digest, replaced.capture.capture_digest);

    let cancelled = crate::hearing_fixture::persist(
        &db,
        actor,
        crate::hearing_fixture::cancellation(&replaced),
    );
    assert!(store.list(db.owner, later_query).unwrap().items.is_empty());
    for kind in [AgendaKind::All, AgendaKind::PrecautionaryHearing] {
        let page = store
            .list(
                db.owner,
                query(later, 1, kind, HearingStatusFilter::Cancelled, None),
            )
            .unwrap();
        assert_eq!(page.items.len(), 1);
        let shown = appointment(&page.items[0]);
        assert_eq!(shown.revision, cancelled.capture.review.result_revision);
        assert_eq!(shown.status, HearingStatus::Cancelled);
        assert_eq!(shown.capture_digest, cancelled.capture.capture_digest);
        assert!(page.complete && page.next_after.is_none());
    }
}

#[test]
fn precautionary_agenda_preserves_closed_reads_and_current_case_isolation() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = crate::hearing_fixture::setup(&mut db);
    let at = scheduled(&command);
    let first_case = db.case;
    let first = crate::hearing_fixture::persist(&db, actor, command);
    let assigned = db.user("paralegal", true);
    let (actor, command) = crate::hearing_fixture::setup(&mut db);
    let second_case = db.case;
    let second = crate::hearing_fixture::persist(&db, actor, command);
    let foreign = db.user("litigator", true);
    let client = db.user("client", true);
    let store = store(&db);
    let q = query(
        at,
        20,
        AgendaKind::PrecautionaryHearing,
        HearingStatusFilter::All,
        None,
    );
    assert_eq!(store.list(db.owner, q).unwrap().items.len(), 2);
    let page = store.list(assigned, q).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(appointment(&page.items[0]).case_id, first_case);
    assert_eq!(
        appointment(&page.items[0]).id,
        first.capture.review.command.hearing_id
    );
    let page = store.list(foreign, q).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(appointment(&page.items[0]).case_id, second_case);
    assert_eq!(
        appointment(&page.items[0]).id,
        second.capture.review.command.hearing_id
    );
    assert!(matches!(
        store.list(client, q),
        Err(ApplicationError::PermissionDenied)
    ));
    db.store()
        .change_administrative_status(
            db.owner,
            first_case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let page = store.list(assigned, q).unwrap();
    assert_eq!(page.items.len(), 1);
    let AgendaItem::PrecautionaryHearing { case, hearing } = &page.items[0] else {
        unreachable!()
    };
    assert_eq!(case.status, CaseAdministrativeStatus::Closed);
    assert_eq!(hearing.capture_digest, first.capture.capture_digest);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&first_case.as_uuid(), &assigned.as_uuid()],
        )
        .unwrap();
    assert!(store.list(assigned, q).unwrap().items.is_empty());
}
