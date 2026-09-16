use domain::identity::{Permission, Role};

#[test]
fn facts_keep_staff_reads_separate_from_management_and_deny_clients() {
    for role in [Role::Owner, Role::Litigator] {
        assert!(role.allows(Permission::ReadProceduralFact));
        assert!(role.allows(Permission::ManageProceduralFact));
    }
    assert!(Role::Paralegal.allows(Permission::ReadProceduralFact));
    assert!(!Role::Paralegal.allows(Permission::ManageProceduralFact));
    assert!(!Role::Client.allows(Permission::ReadProceduralFact));
    assert!(!Role::Client.allows(Permission::ManageProceduralFact));
}
