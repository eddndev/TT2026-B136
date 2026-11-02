//! A Client account is never a case member:
//! docs/adr/0086-bilateral-contract-signing.md.
use crate::member_support;
use application::{
    cases::CaseRepository,
    members::{CaseMemberQuery, MemberError, MemberSelection, MemberStore, UserAccessChange},
    ApplicationError,
};
use domain::identity::{Role, UserId};
use member_support::{snapshot, store, user, Fixture};

fn page(db: &Fixture, selection: MemberSelection, role: Option<Role>) -> Vec<UserId> {
    store(db)
        .list_case_members(
            db.owner,
            db.case,
            CaseMemberQuery::new(db.case, 100, selection, role, None, None).unwrap(),
            db.at,
        )
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.user.id)
        .collect()
}

fn rejected(error: postgres::Error) -> bool {
    error.as_db_error().map(|e| (e.code().code(), e.message()))
        == Some(("23514", "client accounts are never case members"))
}

#[test]
fn the_case_repository_rejects_a_client_assignment_without_writing() {
    let Some(mut db) = Fixture::new() else { return };
    let client = user(&mut db, "client@example.test", "client", true, false);
    let before = snapshot(&mut db);
    assert!(matches!(
        db.store().add_member(db.case, client, db.owner, db.at),
        Err(ApplicationError::InvalidInput(_))
    ));
    assert_eq!(snapshot(&mut db), before);
    let litigator = user(&mut db, "litigator@example.test", "litigator", true, false);
    db.store()
        .add_member(db.case, litigator, db.owner, db.at)
        .unwrap();
}

#[test]
fn available_member_pages_never_offer_a_client_account() {
    let Some(mut db) = Fixture::new() else { return };
    let client = user(&mut db, "client@example.test", "client", true, false);
    let paralegal = user(&mut db, "paralegal@example.test", "paralegal", true, false);
    let available = page(&db, MemberSelection::Available, None);
    assert!(available.contains(&paralegal) && !available.contains(&client));
    assert!(page(&db, MemberSelection::Available, Some(Role::Client)).is_empty());
    assert!(!page(&db, MemberSelection::Assigned, None).contains(&client));
}

#[test]
fn storage_rejects_client_memberships_and_the_service_rejects_member_role_changes() {
    let Some(mut db) = Fixture::new() else { return };
    let client = user(&mut db, "client@example.test", "client", true, false);
    let insert = "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)";
    let error = db
        .admin
        .execute(insert, &[&db.case.as_uuid(), &client.as_uuid()])
        .unwrap_err();
    assert!(rejected(error));
    let error = db
        .runtime()
        .execute(insert, &[&db.case.as_uuid(), &client.as_uuid()])
        .unwrap_err();
    assert!(rejected(error));
    let member = user(&mut db, "member@example.test", "litigator", true, true);
    let before = snapshot(&mut db);
    let change = UserAccessChange::new(0, Role::Client, true).unwrap();
    assert!(matches!(
        store(&db).change_access(db.owner, member, change, db.at),
        Err(ApplicationError::Member(MemberError::Invalid(_)))
    ));
    assert_eq!(snapshot(&mut db), before);
    let free = user(&mut db, "free@example.test", "litigator", true, false);
    let change = UserAccessChange::new(0, Role::Client, true).unwrap();
    let changed = store(&db)
        .change_access(db.owner, free, change, db.at)
        .unwrap();
    assert_eq!(changed.role, Role::Client);
}

#[test]
fn migration_and_startup_refuse_an_existing_client_membership_without_deleting_it() {
    let Some(mut db) = Fixture::new() else { return };
    let client = user(&mut db, "member@example.test", "litigator", true, true);
    db.admin
        .execute(
            "UPDATE users SET role='client',revision=revision+1,
            auth_generation=auth_generation+1 WHERE id=$1",
            &[&client.as_uuid()],
        )
        .unwrap();
    assert!(infrastructure::initialize_database(&db.admin_url, &db.role).is_err());
    let kept: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_memberships WHERE user_id=$1",
            &[&client.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(kept, 1);
    assert!(infrastructure::PostgresMemberStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::SystemClock)
    )
    .is_err());
}

#[test]
fn startup_rejects_a_removed_client_membership_guard() {
    let Some(db) = Fixture::new() else { return };
    store(&db);
    let mut admin = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    admin
        .batch_execute("DROP TRIGGER member_client_guard ON case_memberships")
        .unwrap();
    assert!(infrastructure::PostgresMemberStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::SystemClock)
    )
    .is_err());
    db.migrate();
    store(&db);
}
