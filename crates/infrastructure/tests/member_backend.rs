mod member_support;
use application::members::{
    CaseMemberQuery, MemberError, MemberSelection, MemberStore, UserAccessChange, UserQuery,
    UserStatusFilter,
};
use application::ApplicationError;
use domain::identity::{Role, UserId};
use member_support::{store, user, Fixture};

#[test]
fn owner_directory_filters_before_pagination_and_denies_other_roles_before_lookup() {
    let Some(mut db) = Fixture::new() else { return };
    let first = user(&mut db, "team_one@example.test", "paralegal", true, false);
    let second = user(&mut db, "team_two@example.test", "paralegal", true, false);
    user(
        &mut db,
        "team_inactive@example.test",
        "paralegal",
        false,
        false,
    );
    user(&mut db, "team_other@example.test", "litigator", true, false);
    user(
        &mut db,
        "teamxprefix@example.test",
        "paralegal",
        true,
        false,
    );
    let store = store(&db);
    let query = UserQuery::new(
        1,
        UserStatusFilter::Active,
        Some(Role::Paralegal),
        Some("team_"),
        None,
    )
    .unwrap();
    let page = store.list(db.owner, query, db.at).unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.has_more);
    let query = UserQuery::new(
        1,
        UserStatusFilter::Active,
        Some(Role::Paralegal),
        Some("team_"),
        page.next_cursor.as_deref(),
    )
    .unwrap();
    let next = store.list(db.owner, query, db.at).unwrap();
    assert!(!next.has_more);
    assert!(next.next_cursor.is_none());
    let mut expected = vec![first.as_uuid(), second.as_uuid()];
    expected.sort();
    assert_eq!(
        vec![page.items[0].id.as_uuid(), next.items[0].id.as_uuid()],
        expected
    );
    for role in ["litigator", "paralegal", "client"] {
        let actor = db.user(role, false);
        assert!(matches!(
            store.get(actor, UserId::new(), db.at),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn assigned_members_include_inactive_accounts_and_available_excludes_members() {
    let Some(mut db) = Fixture::new() else { return };
    let assigned = user(&mut db, "assigned@example.test", "paralegal", true, true);
    let inactive = user(&mut db, "inactive@example.test", "client", false, true);
    let available = user(&mut db, "available@example.test", "litigator", true, false);
    user(&mut db, "disabled@example.test", "litigator", false, false);
    let store = store(&db);
    let page = store
        .list_case_members(
            db.owner,
            db.case,
            CaseMemberQuery::new(db.case, 100, MemberSelection::Assigned, None, None, None)
                .unwrap(),
            db.at,
        )
        .unwrap();
    let mut expected = vec![assigned.as_uuid(), inactive.as_uuid()];
    expected.sort();
    assert_eq!(
        page.items
            .iter()
            .map(|item| item.user.id.as_uuid())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(page.items.iter().all(|item| item.assigned_at.is_some()));
    let available_page = store
        .list_case_members(
            db.owner,
            db.case,
            CaseMemberQuery::new(
                db.case,
                100,
                MemberSelection::Available,
                Some(Role::Litigator),
                None,
                None,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(available_page.items.len(), 1);
    assert_eq!(available_page.items[0].user.id, available);
    assert!(available_page.items[0].assigned_at.is_none());
}

#[test]
fn access_change_uses_cas_preserves_memberships_and_advances_generation_once() {
    let Some(mut db) = Fixture::new() else { return };
    let target = user(&mut db, "target@example.test", "litigator", true, true);
    let store = store(&db);
    let change = UserAccessChange::new(0, Role::Paralegal, false).unwrap();
    let result = store
        .change_access(db.owner, target, change, db.at)
        .unwrap();
    assert!(!result.active);
    assert_eq!(result.role, Role::Paralegal);
    assert_eq!(result.revision, 1);
    let row = db.admin.query_one("SELECT auth_generation,(SELECT count(*) FROM case_memberships WHERE user_id=$1) FROM users WHERE id=$1", &[&target.as_uuid()]).unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    assert_eq!(row.get::<_, i64>(1), 1);
    let before = member_support::snapshot(&mut db);
    assert_eq!(
        store
            .change_access(
                db.owner,
                target,
                UserAccessChange::new(1, Role::Paralegal, false).unwrap(),
                db.at
            )
            .unwrap(),
        result
    );
    assert_eq!(member_support::snapshot(&mut db), before);
    assert!(matches!(
        store.change_access(
            db.owner,
            target,
            UserAccessChange::new(0, Role::Paralegal, false).unwrap(),
            db.at
        ),
        Err(ApplicationError::Member(MemberError::RevisionConflict))
    ));
    let restored = store
        .change_access(
            db.owner,
            target,
            UserAccessChange::new(1, Role::Paralegal, true).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(restored.revision, 2);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT auth_generation FROM users WHERE id=$1",
                &[&target.as_uuid()]
            )
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}

#[test]
fn last_owner_is_protected_but_self_demotion_with_another_owner_commits() {
    let Some(mut db) = Fixture::new() else { return };
    let store = store(&db);
    for (role, active) in [(Role::Owner, false), (Role::Paralegal, true)] {
        assert!(matches!(
            store.change_access(
                db.owner,
                db.owner,
                UserAccessChange::new(0, role, active).unwrap(),
                db.at
            ),
            Err(ApplicationError::Member(MemberError::LastActiveOwner))
        ));
    }
    user(&mut db, "backup@example.test", "owner", true, false);
    let changed = store
        .change_access(
            db.owner,
            db.owner,
            UserAccessChange::new(0, Role::Paralegal, true).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(changed.role, Role::Paralegal);
    assert_eq!(changed.revision, 1);
    assert!(matches!(
        store.get(db.owner, db.owner, db.at),
        Err(ApplicationError::PermissionDenied)
    ));
}
