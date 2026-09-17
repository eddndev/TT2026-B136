use domain::identity::{Permission, Role};

#[test]
fn deadline_input_reads_allow_staff_roles_and_deny_clients() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        assert!(role.allows(Permission::ReadDeadlineInputs));
    }
    assert!(!Role::Client.allows(Permission::ReadDeadlineInputs));
}
