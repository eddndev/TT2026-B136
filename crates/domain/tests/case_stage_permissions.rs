use domain::identity::{Permission, Role};

#[test]
fn staff_stage_reads_and_changes_have_separate_permissions() {
    for role in [Role::Owner, Role::Litigator] {
        assert!(role.allows(Permission::ReadCaseStage));
        assert!(role.allows(Permission::ManageCaseStage));
    }
    assert!(Role::Paralegal.allows(Permission::ReadCaseStage));
    assert!(!Role::Paralegal.allows(Permission::ManageCaseStage));
    assert!(!Role::Client.allows(Permission::ReadCaseStage));
    assert!(!Role::Client.allows(Permission::ManageCaseStage));
}
