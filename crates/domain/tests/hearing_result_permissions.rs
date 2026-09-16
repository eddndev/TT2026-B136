use domain::identity::{Permission, Role};

#[test]
fn recording_results_and_reading_history_have_separate_permissions() {
    for role in [Role::Owner, Role::Litigator] {
        assert!(role.allows(Permission::ReadHearingResult));
        assert!(role.allows(Permission::ManageHearingResult));
    }
    assert!(Role::Paralegal.allows(Permission::ReadHearingResult));
    assert!(!Role::Paralegal.allows(Permission::ManageHearingResult));
    assert!(!Role::Client.allows(Permission::ReadHearingResult));
    assert!(!Role::Client.allows(Permission::ManageHearingResult));
}
