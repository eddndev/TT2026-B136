use crate::{agenda_backend_support as agenda, resource_hearing_database_support as own};
use application::{
    agenda::*,
    cases::{CaseRepository, CaseRevisionExpectation},
    hearings::HearingStatusFilter,
    resource_hearings::*,
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    hearings::*,
    identity::Role,
    resource_hearings::{ResourceHearingValues, ResourceHearingValuesInput},
};
use time::{Duration, OffsetDateTime};

#[path = "agenda_resource_hearing_closure.rs"]
mod closure;

fn query(
    at: OffsetDateTime,
    limit: u32,
    kind: AgendaKind,
    status: HearingStatusFilter,
    after: Option<AgendaCursor>,
) -> AgendaQuery {
    AgendaQuery::new(
        limit,
        at.to_offset(time::UtcOffset::UTC),
        at.to_offset(time::UtcOffset::UTC) + Duration::seconds(1),
        kind,
        status,
        after,
    )
    .unwrap()
}
fn at(command: &mut ResourceHearingCommand, time: OffsetDateTime) {
    let v = &command.values;
    command.values = ResourceHearingValues::new(ResourceHearingValuesInput {
        kind: v.kind(),
        scheduled_at: HearingTime::new(time).unwrap(),
        modality: v.modality(),
        venue: v.venue().clone(),
        note: v.note().cloned(),
        participants: v.participants().to_vec(),
        scheduling_basis: v.scheduling_basis().clone(),
    })
    .unwrap();
}

#[test]
fn three_families_with_identical_uuid_and_time_preserve_cursor_order() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (_, mut command) = own::setup(&mut db);
    let (_, deadline) = agenda::accepted(&db, 123);
    let time = deadline.calculation.result.due_at().unwrap();
    let hearing = agenda::hearing_at(&db, deadline.id.as_uuid(), time);
    command.hearing_id =
        domain::resource_hearings::ResourceHearingId::from_uuid(deadline.id.as_uuid());
    at(&mut command, time);
    let created = own::submit(&db, command);
    let store = agenda::store(&db);
    let mut cursor = None;
    for kind in [
        AgendaItemKind::Hearing,
        AgendaItemKind::Deadline,
        AgendaItemKind::ResourceHearing,
    ] {
        let q = query(
            time,
            1,
            AgendaKind::All,
            HearingStatusFilter::Scheduled,
            cursor,
        );
        let page = store.list(db.owner, q).unwrap();
        assert_eq!(page.items.len(), 1);
        let item = &page.items[0];
        assert_eq!(item.key().unwrap().kind(), kind);
        assert_eq!(item.key().unwrap().id(), deadline.id.as_uuid());
        if let AgendaItem::ResourceHearing { case, hearing: own } = item {
            assert_eq!(case.case_id, db.case);
            assert_eq!(own.resource_id, created.origin.resource_id);
            assert_eq!(own.revision, created.hearing.revision);
            assert_eq!(own.association_id, created.origin.association_id);
            assert_eq!(own.capture_digest, created.origin.capture_digest);
            assert_eq!(
                own.scheduled_at.value(),
                created.hearing.review.command.values.scheduled_at().value()
            );
        }
        assert_eq!(page.complete, kind == AgendaItemKind::ResourceHearing);
        cursor = page.next_after;
    }
    assert!(cursor.is_none());
    assert_eq!(
        hearing.snapshot.id.as_uuid(),
        created.origin.hearing_id.as_uuid()
    );
    let ordinary = store
        .list(
            db.owner,
            query(
                time,
                20,
                AgendaKind::Hearing,
                HearingStatusFilter::All,
                None,
            ),
        )
        .unwrap();
    assert_eq!(ordinary.items.len(), 1);
    let cancelled = store
        .list(
            db.owner,
            query(
                time,
                20,
                AgendaKind::All,
                HearingStatusFilter::Cancelled,
                None,
            ),
        )
        .unwrap();
    assert_eq!(cancelled.items.len(), 1);
    assert!(matches!(cancelled.items[0], AgendaItem::Deadline { .. }));
    assert_eq!(agenda::audits(&mut db), 5);
}

#[test]
fn resource_appointments_remain_after_unlink_archive_and_case_close() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (captures, command) = own::setup(&mut db);
    let time = command.values.scheduled_at().value();
    let created = own::submit(&db, command);
    crate::resource_activity_support::persist(
        &crate::resource_activity_support::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        crate::resource_activity_support::unlink(&created.association, captures.head.revision),
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
    let page = agenda::store(&db)
        .list(
            db.owner,
            query(
                time,
                20,
                AgendaKind::ResourceHearing,
                HearingStatusFilter::All,
                None,
            ),
        )
        .unwrap();
    assert_eq!(page.items.len(), 1);
    let AgendaItem::ResourceHearing { case, hearing } = &page.items[0] else {
        panic!("resource hearing")
    };
    assert_eq!(case.status, CaseAdministrativeStatus::Closed);
    assert_eq!(hearing.capture_digest, created.origin.capture_digest);
    assert_eq!(hearing.association_id, created.origin.association_id);
}

#[test]
fn resource_agenda_enforces_membership_revocation_and_client_denial() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (_, command) = own::setup(&mut db);
    let time = command.values.scheduled_at().value();
    own::submit(&db, command);
    let assigned = db.user("paralegal", true);
    let foreign = db.user("litigator", false);
    let client = db.user("client", true);
    let store = agenda::store(&db);
    let q = query(
        time,
        20,
        AgendaKind::ResourceHearing,
        HearingStatusFilter::Scheduled,
        None,
    );
    assert_eq!(store.list(assigned, q).unwrap().items.len(), 1);
    assert!(store.list(foreign, q).unwrap().items.is_empty());
    assert!(matches!(
        store.list(client, q),
        Err(ApplicationError::PermissionDenied)
    ));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&assigned.as_uuid()],
        )
        .unwrap();
    assert!(store.list(assigned, q).unwrap().items.is_empty());
}

#[test]
fn inconsistent_resource_agenda_origin_or_header_is_not_silently_omitted() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (_, command) = own::setup(&mut db);
    let time = command.values.scheduled_at().value();
    own::submit(&db, command);
    let store = agenda::store(&db);
    let q = query(
        time,
        20,
        AgendaKind::ResourceHearing,
        HearingStatusFilter::Scheduled,
        None,
    );
    db.admin.batch_execute("SET session_replication_role=replica;
        UPDATE case_resource_hearing_revisions SET values_view=jsonb_set(values_view,'{modality}',to_jsonb('videoconference'::text));
        SET session_replication_role=origin").unwrap();
    assert!(store.list(db.owner, q).is_err());
    assert_eq!(agenda::audits(&mut db), 0);
}

#[test]
fn missing_resource_origin_and_failed_read_audit_do_not_return_agenda_data() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (_, command) = own::setup(&mut db);
    let time = command.values.scheduled_at().value();
    own::submit(&db, command);
    let store = agenda::store(&db);
    let q = query(
        time,
        20,
        AgendaKind::ResourceHearing,
        HearingStatusFilter::Scheduled,
        None,
    );
    db.admin.batch_execute("CREATE FUNCTION reject_agenda_read() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected read failure'; END; $$;
        CREATE TRIGGER reject_agenda_read BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='agenda.read') EXECUTE FUNCTION reject_agenda_read()").unwrap();
    assert!(store.list(db.owner, q).is_err());
    assert_eq!(agenda::audits(&mut db), 0);
    db.admin.batch_execute("DROP TRIGGER reject_agenda_read ON audit_events;
        DROP FUNCTION reject_agenda_read(); SET session_replication_role=replica;
        DELETE FROM audit_events WHERE action='resource_hearing.registered'; SET session_replication_role=origin").unwrap();
    assert!(store.list(db.owner, q).is_err());
    assert_eq!(agenda::audits(&mut db), 0);
}
