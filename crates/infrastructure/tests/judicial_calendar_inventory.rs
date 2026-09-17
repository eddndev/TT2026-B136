mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
mod judicial_calendar_inventory_support;
use application::judicial_calendars::*;
use case_administration_support::Fixture;
use domain::identity::Role;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use judicial_calendar_database_support::{persist, publish, replace, retire, service, values};
use judicial_calendar_inventory_support::{forged_revision, insert};
use std::sync::Arc;

fn rejects(db: &Fixture) {
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}
#[test]
fn startup_rejects_calendar_root_without_first_revision() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendars DISABLE TRIGGER ALL;
        INSERT INTO judicial_calendars(id) VALUES('00000000-0000-0000-0000-000000000000');
        ALTER TABLE judicial_calendars ENABLE TRIGGER ALL",
        )
        .unwrap();
    rejects(&db);
}
#[test]
fn startup_checks_scope_history_for_the_nil_uuid() {
    let Some(mut db) = Fixture::new() else { return };
    let svc = service(&db, db.owner, Role::Owner);
    let mut command = publish();
    command.calendar_id = JudicialCalendarId::from_uuid(uuid::Uuid::nil());
    let first = persist(&svc, command);
    let forged = forged_revision(
        &mut db,
        &first,
        &replace(&first),
        &values("Illicit scope", "Unresolved"),
    );
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    insert(&mut db, &forged);
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions ENABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    rejects(&db);
}
#[test]
fn startup_rejects_missing_intermediate_calendar_history() {
    let Some(mut db) = Fixture::new() else { return };
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, publish());
    let second = persist(&svc, replace(&first));
    persist(&svc, retire(&second));
    db.admin
        .batch_execute("ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM judicial_calendar_revisions WHERE calendar_id=$1 AND revision=2",
            &[&first.id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE judicial_calendar_revisions ENABLE TRIGGER ALL")
        .unwrap();
    rejects(&db);
}
#[test]
fn reads_and_inventory_reject_retirement_with_changed_values() {
    let Some(mut db) = Fixture::new() else { return };
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, publish());
    let forged = forged_revision(
        &mut db,
        &first,
        &retire(&first),
        &values("Calendar", "Illicit replacement during retirement"),
    );
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    insert(&mut db, &forged);
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions ENABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    assert!(svc.get("session", first.id, None).is_err());
    rejects(&db);
}
#[test]
fn inventory_inspects_revisions_after_the_first_page() {
    let Some(mut db) = Fixture::new() else { return };
    let svc = service(&db, db.owner, Role::Owner);
    let mut last = persist(&svc, publish());
    for _ in 0..64 {
        last = persist(&svc, replace(&last));
    }
    let forged = forged_revision(
        &mut db,
        &last,
        &retire(&last),
        &values("Calendar", "Corruption beyond inventory page one"),
    );
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    insert(&mut db, &forged);
    db.admin
        .batch_execute(
            "ALTER TABLE judicial_calendar_revisions ENABLE TRIGGER judicial_calendar_sequence",
        )
        .unwrap();
    rejects(&db);
}

#[test]
fn startup_rejects_calendar_history_with_a_missing_recorded_actor() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("owner", false);
    let svc = service(&db, actor, Role::Owner);
    persist(&svc, publish());
    db.admin
        .batch_execute("ALTER TABLE users DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute("DELETE FROM users WHERE id=$1", &[&actor.as_uuid()])
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE users ENABLE TRIGGER ALL")
        .unwrap();
    rejects(&db);
}
