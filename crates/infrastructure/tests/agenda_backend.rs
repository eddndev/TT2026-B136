mod agenda_backend_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;

use agenda_backend_support::*;
use application::{agenda::*, cases::CaseRepository, ApplicationError};
use domain::identity::Role;
use procedural_fact_backend_support as facts;

#[test]
fn combined_page_preserves_both_families_with_the_same_instant_and_uuid() {
    let Some(mut db) = fixture() else { return };
    let (_, deadline) = accepted(&db, 1);
    let hearing = hearing_at(
        &db,
        deadline.id.as_uuid(),
        deadline.calculation.result.due_at().unwrap(),
    );
    let store = store(&db);
    let before = unchanged_resources(&mut db);
    let first_query = query(1, AgendaKind::All, None);
    let first = store.list(db.owner, first_query).unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(matches!(&first.items[0], AgendaItem::Hearing(value)
        if value.id == hearing.snapshot.id));
    assert!(!first.complete);
    let second = store
        .list(db.owner, query(1, AgendaKind::All, first.next_after))
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(
        matches!(&second.items[0], AgendaItem::Deadline { case, deadline: value }
        if case.case_id == db.case && value.id == deadline.id)
    );
    assert!(second.complete);
    assert!(second.next_after.is_none());
    assert_eq!(first.checked_at, db.at);
    assert_eq!(second.checked_at, db.at);
    assert_eq!(unchanged_resources(&mut db), before);
    assert_eq!(audits(&mut db), 2);
}

#[test]
fn changed_follow_source_is_omitted_without_dispatch_or_history_mutation() {
    let Some(mut db) = fixture() else { return };
    let (source, deadline) = accepted(&db, 2);
    let store = store(&db);
    let first = store
        .list(db.owner, query(20, AgendaKind::Deadline, None))
        .unwrap();
    assert_eq!(first.items.len(), 1);
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::correct(&source),
    );
    let before = unchanged_resources(&mut db);
    let changed = store
        .list(db.owner, query(20, AgendaKind::Deadline, None))
        .unwrap();
    assert!(changed.items.is_empty());
    assert!(changed.complete);
    assert!(changed.next_after.is_none());
    assert!(deadline.calculation.result.due_at().is_some());
    assert_eq!(unchanged_resources(&mut db), before);
    assert_eq!(audits(&mut db), 2);
}

#[test]
fn membership_filters_both_families_before_limit_and_revocation_is_rechecked() {
    let Some(mut db) = fixture() else { return };
    let (_, foreign) = accepted(&db, 1);
    hearing_at(
        &db,
        foreign.id.as_uuid(),
        foreign.calculation.result.due_at().unwrap(),
    );
    hearing_database_support::complete(&mut db);
    let (_, ours) = accepted(&db, 20);
    hearing_at(
        &db,
        ours.id.as_uuid(),
        ours.calculation.result.due_at().unwrap(),
    );
    let litigator = db.user("litigator", true);
    let paralegal = db.user("paralegal", true);
    let client = db.user("client", true);
    let store = store(&db);
    for actor in [litigator, paralegal] {
        let page = store.list(actor, query(1, AgendaKind::All, None)).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(matches!(&page.items[0], AgendaItem::Hearing(value)
            if value.case_id == db.case && value.id.as_uuid() == ours.id.as_uuid()));
        assert!(!page.complete);
    }
    assert!(matches!(
        store.list(client, query(20, AgendaKind::All, None)),
        Err(ApplicationError::PermissionDenied)
    ));
    let global = store
        .list(db.owner, query(20, AgendaKind::All, None))
        .unwrap();
    assert_eq!(global.items.len(), 4);
    db.store()
        .remove_member(db.case, litigator, db.owner, db.at)
        .unwrap();
    let revoked = store
        .list(litigator, query(20, AgendaKind::All, None))
        .unwrap();
    assert!(revoked.items.is_empty());
    assert!(revoked.complete);
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.list(paralegal, query(20, AgendaKind::All, None)),
        Err(ApplicationError::InvalidSession)
    ));
}
