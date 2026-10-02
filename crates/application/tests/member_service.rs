#![allow(dead_code)]

use crate::{case_support, member_support};
use application::members::*;
use application::ApplicationError;
use case_support::{identity, instant, MockIdentity};
use member_support::{page, query, service, user, MockStore};

#[test]
fn owner_reads_a_verified_directory_page_with_exact_continuation() {
    let (identity, actor) = identity(Role::Owner, 2);
    let requested = UserQuery::new(
        1,
        UserStatusFilter::Active,
        Some(Role::Paralegal),
        Some("staff"),
        None,
    )
    .unwrap();
    let row = user(UserId::new());
    let expected = UserPage {
        items: vec![row.clone()],
        has_more: true,
        next_cursor: Some(requested.cursor_after(row.id)),
    };
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_list()
        .withf(move |id, q, at| {
            *id == actor.id
                && q.limit() == 1
                && q.email_prefix() == Some("staff")
                && *at == instant()
        })
        .times(1)
        .returning(move |_, _, _| Ok(returned.clone()));
    assert_eq!(
        service(store, identity).list("session", requested).unwrap(),
        expected
    );
}

#[test]
fn every_non_owner_is_denied_before_directory_target_or_case_lookup() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (identity, _) = identity(role, 4);
        let service = service(MockStore::new(), identity);
        let case = CaseId::new();
        let commands = [
            service.list("session", query()).map(|_| ()),
            service.get("session", UserId::new()).map(|_| ()),
            service
                .change_access(
                    "session",
                    UserId::new(),
                    UserAccessChange::new(0, Role::Client, false).unwrap(),
                )
                .map(|_| ()),
            service
                .list_case_members(
                    "session",
                    case,
                    CaseMemberQuery::new(case, 1, MemberSelection::Assigned, None, None, None)
                        .unwrap(),
                )
                .map(|_| ()),
        ];
        for result in commands {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
    }
}

#[test]
fn directory_delivery_reauthenticates_the_entire_principal() {
    let (_, actor) = identity(Role::Owner, 0);
    let mut changed = actor.clone();
    changed.email = "different@example.com".into();
    let mut identity = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(changed));
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .returning(|_, _, _| Ok(page(Vec::new())));
    assert!(matches!(
        service(store, identity).list("session", query()),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn invalid_directory_projections_are_stored_errors_not_input_errors() {
    let row = user(UserId::new());
    let mut wrong_email = row.clone();
    wrong_email.email = "user\n@example.com".into();
    let mut exhausted = row.clone();
    exhausted.revision = i64::MAX as u64 + 1;
    for returned in [
        page(vec![row.clone(), row.clone()]),
        page(vec![wrong_email]),
        page(vec![exhausted]),
        UserPage {
            items: vec![row.clone()],
            has_more: true,
            next_cursor: None,
        },
    ] {
        let (identity, _) = identity(Role::Owner, 1);
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .returning(move |_, _, _| Ok(returned.clone()));
        assert!(matches!(
            service(store, identity).list("session", query()),
            Err(ApplicationError::Member(MemberError::Stored(_)))
        ));
    }
}

#[test]
fn case_selector_preserves_inactive_assignments_and_only_active_available_users() {
    for selection in [MemberSelection::Assigned, MemberSelection::Available] {
        let (identity, _) = identity(Role::Owner, 2);
        let case = CaseId::new();
        let mut row = user(UserId::new());
        row.active = selection == MemberSelection::Available;
        let expected = CaseMemberPage {
            case_id: case,
            items: vec![CaseMemberItem {
                user: row,
                assigned_at: (selection == MemberSelection::Assigned).then_some(instant()),
            }],
            has_more: false,
            next_cursor: None,
        };
        let returned = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_list_case_members()
            .times(1)
            .returning(move |_, _, _, _| Ok(returned.clone()));
        let query = CaseMemberQuery::new(case, 10, selection, None, None, None).unwrap();
        assert_eq!(
            service(store, identity)
                .list_case_members("session", case, query)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn user_detail_and_case_selector_reject_a_different_response_identity() {
    let (identity, _) = identity(Role::Owner, 1);
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .returning(|_, _, _| Ok(user(UserId::new())));
    assert!(matches!(
        service(store, identity).get("session", UserId::new()),
        Err(ApplicationError::Member(MemberError::Stored(_)))
    ));

    let (identity, _) = case_support::identity(Role::Owner, 1);
    let case = CaseId::new();
    let mut store = MockStore::new();
    store
        .expect_list_case_members()
        .times(1)
        .returning(|_, _, _, _| {
            Ok(CaseMemberPage {
                case_id: CaseId::new(),
                items: Vec::new(),
                has_more: false,
                next_cursor: None,
            })
        });
    let query =
        CaseMemberQuery::new(case, 10, MemberSelection::Assigned, None, None, None).unwrap();
    assert!(matches!(
        service(store, identity).list_case_members("session", case, query),
        Err(ApplicationError::Member(MemberError::Stored(_)))
    ));
}

#[test]
fn self_deactivation_returns_the_committed_result_without_post_commit_authentication() {
    let (identity, actor) = identity(Role::Owner, 2);
    let mut expected = user(actor.id);
    expected.email = actor.email.clone();
    expected.role = Role::Owner;
    expected.active = false;
    expected.revision = 8;
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_change_access()
        .withf(move |a, id, change, _| {
            *a == actor.id && *id == actor.id && change.expected_revision() == 7 && !change.active()
        })
        .times(1)
        .returning(move |_, _, _, _| Ok(returned.clone()));
    let change = UserAccessChange::new(7, Role::Owner, false).unwrap();
    assert_eq!(
        service(store, identity)
            .change_access("session", expected.id, change)
            .unwrap(),
        expected
    );
}

#[test]
fn access_conflicts_and_failed_audit_are_propagated_without_fabricated_success() {
    for expected in [
        MemberError::RevisionConflict,
        MemberError::LastActiveOwner,
        MemberError::AccessVersionExhausted,
    ] {
        let (identity, _) = identity(Role::Owner, 2);
        let returned = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_change_access()
            .times(1)
            .returning(move |_, _, _, _| Err(returned.clone().into()));
        let change = UserAccessChange::new(1, Role::Client, false).unwrap();
        assert!(
            matches!(service(store, identity).change_access("session", UserId::new(), change),
            Err(ApplicationError::Member(error)) if error == expected)
        );
    }
    let (identity, _) = identity(Role::Owner, 2);
    let mut store = MockStore::new();
    store
        .expect_change_access()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::Port("audit unavailable".into())));
    assert!(matches!(
        service(store, identity).change_access(
            "session",
            UserId::new(),
            UserAccessChange::new(0, Role::Client, true).unwrap()
        ),
        Err(ApplicationError::Port(_))
    ));
}

#[test]
fn unchanged_access_can_return_the_same_revision_without_an_invented_receipt() {
    let (identity, _) = identity(Role::Owner, 2);
    let expected = user(UserId::new());
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_change_access()
        .times(1)
        .returning(move |_, _, _, _| Ok(returned.clone()));
    let change = UserAccessChange::new(expected.revision, expected.role, expected.active).unwrap();
    assert_eq!(
        service(store, identity)
            .change_access("session", expected.id, change)
            .unwrap(),
        expected
    );
}

#[test]
fn access_result_must_match_target_values_and_a_possible_revision() {
    for mismatch in 0..3 {
        let (identity, _) = identity(Role::Owner, 2);
        let target = UserId::new();
        let mut returned = user(target);
        match mismatch {
            0 => returned.id = UserId::new(),
            1 => returned.active = false,
            _ => returned.revision = 10,
        }
        let mut store = MockStore::new();
        store
            .expect_change_access()
            .times(1)
            .returning(move |_, _, _, _| Ok(returned.clone()));
        let change = UserAccessChange::new(7, Role::Paralegal, true).unwrap();
        assert!(matches!(
            service(store, identity).change_access("session", target, change),
            Err(ApplicationError::Member(MemberError::Stored(_)))
        ));
    }
}
