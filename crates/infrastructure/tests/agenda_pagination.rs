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
use application::{agenda::*, deadlines::*};
use deadline_backend_support as dl;
use domain::identity::Role;
use procedural_fact_backend_support as facts;
use uuid::Uuid;

#[test]
fn empty_incomplete_page_advances_past_one_hundred_changed_candidates() {
    let Some(mut db) = fixture() else { return };
    let profile = dl::profile(&db);
    let changed_source = dl::source(&db);
    let current_source = dl::source(&db);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    for id in 1..=101 {
        let source = if id <= 100 {
            &changed_source
        } else {
            &current_source
        };
        let mut command = dl::command(&db, &profile, source);
        command.deadline_id = DeadlineId::from_uuid(Uuid::from_u128(id));
        dl::persist(
            &workflow,
            db.case,
            dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
        );
    }
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::correct(&changed_source),
    );
    let store = store(&db);
    let before = unchanged_resources(&mut db);
    let first = store
        .list(db.owner, query(1, AgendaKind::Deadline, None))
        .unwrap();
    assert!(first.items.is_empty());
    assert!(!first.complete);
    let cursor = first.next_after.unwrap();
    assert_eq!(cursor.id(), Uuid::from_u128(100));
    assert_eq!(cursor.kind(), AgendaItemKind::Deadline);
    let second = store
        .list(db.owner, query(1, AgendaKind::Deadline, Some(cursor)))
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(
        matches!(&second.items[0], AgendaItem::Deadline { deadline, .. }
        if deadline.id.as_uuid() == Uuid::from_u128(101))
    );
    assert!(second.complete);
    assert!(second.next_after.is_none());
    assert_eq!(unchanged_resources(&mut db), before);
    assert_eq!(audits(&mut db), 2);
}
