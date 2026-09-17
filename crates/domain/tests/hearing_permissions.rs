use domain::identity::{Permission, Role};

#[test]
fn scheduling_and_reading_hearings_have_separate_staff_permissions() {
    for role in [Role::Owner, Role::Litigator] {
        assert!(role.allows(Permission::ReadHearing));
        assert!(role.allows(Permission::ManageHearing));
    }
    assert!(Role::Paralegal.allows(Permission::ReadHearing));
    assert!(!Role::Paralegal.allows(Permission::ManageHearing));
    assert!(!Role::Client.allows(Permission::ReadHearing));
    assert!(!Role::Client.allows(Permission::ManageHearing));
}
