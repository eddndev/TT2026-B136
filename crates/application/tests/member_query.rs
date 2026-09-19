use application::members::*;

#[test]
fn directory_limits_prefixes_and_access_revisions_are_bounded() {
    for limit in [0, 101, u32::MAX] {
        assert!(UserQuery::new(limit, UserStatusFilter::All, None, None, None).is_err());
        assert!(CaseMemberQuery::new(
            CaseId::new(),
            limit,
            MemberSelection::Assigned,
            None,
            None,
            None
        )
        .is_err());
    }
    for prefix in ["a\nb", "a\0b", "a\x7fb", "\u{e9}@example.com"] {
        assert!(UserQuery::new(1, UserStatusFilter::Active, None, Some(prefix), None).is_err());
    }
    assert!(UserQuery::new(1, UserStatusFilter::All, None, Some(&"a".repeat(255)), None).is_err());
    assert!(UserAccessChange::new(i64::MAX as u64 + 1, Role::Client, false).is_err());
    assert!(UserAccessChange::new(i64::MAX as u64, Role::Owner, true).is_ok());
}

#[test]
fn prefixes_are_normalized_but_sql_metacharacters_remain_literal() {
    let query = UserQuery::new(
        50,
        UserStatusFilter::All,
        Some(Role::Paralegal),
        Some("  A_%\\@Example  "),
        None,
    )
    .unwrap();
    assert_eq!(query.email_prefix(), Some("a_%\\@example"));
    assert_eq!(query.role(), Some(Role::Paralegal));
    assert_eq!(query.status(), UserStatusFilter::All);
    assert_eq!(
        UserQuery::new(1, UserStatusFilter::All, None, Some("  "), None)
            .unwrap()
            .email_prefix(),
        None
    );
}

#[test]
fn directory_cursor_roundtrips_and_binds_every_normalized_filter() {
    let id = UserId::new();
    let query = UserQuery::new(
        2,
        UserStatusFilter::Inactive,
        Some(Role::Client),
        Some("client+"),
        None,
    )
    .unwrap();
    let cursor = query.cursor_after(id);
    assert!(cursor.is_ascii() && cursor.len() <= 768);
    let resumed = UserQuery::new(
        100,
        UserStatusFilter::Inactive,
        Some(Role::Client),
        Some("CLIENT+"),
        Some(&cursor),
    )
    .unwrap();
    assert_eq!(resumed.after_id(), Some(id));
    for (status, role, prefix) in [
        (
            UserStatusFilter::Active,
            Some(Role::Client),
            Some("client+"),
        ),
        (
            UserStatusFilter::Inactive,
            Some(Role::Owner),
            Some("client+"),
        ),
        (UserStatusFilter::Inactive, Some(Role::Client), None),
    ] {
        assert!(UserQuery::new(2, status, role, prefix, Some(&cursor)).is_err());
    }
    assert!(UserQuery::new(2, UserStatusFilter::All, None, None, Some("invalid")).is_err());
}

#[test]
fn case_cursor_binds_case_selection_and_cannot_be_used_as_directory_cursor() {
    let case = CaseId::new();
    let id = UserId::new();
    let query = CaseMemberQuery::new(
        case,
        1,
        MemberSelection::Assigned,
        None,
        Some("staff"),
        None,
    )
    .unwrap();
    let cursor = query.cursor_after(id);
    assert_eq!(
        CaseMemberQuery::new(
            case,
            5,
            MemberSelection::Assigned,
            None,
            Some("staff"),
            Some(&cursor)
        )
        .unwrap()
        .after_id(),
        Some(id)
    );
    assert!(CaseMemberQuery::new(
        CaseId::new(),
        1,
        MemberSelection::Assigned,
        None,
        Some("staff"),
        Some(&cursor)
    )
    .is_err());
    assert!(CaseMemberQuery::new(
        case,
        1,
        MemberSelection::Available,
        None,
        Some("staff"),
        Some(&cursor)
    )
    .is_err());
    assert!(UserQuery::new(1, UserStatusFilter::All, None, Some("staff"), Some(&cursor)).is_err());
}
