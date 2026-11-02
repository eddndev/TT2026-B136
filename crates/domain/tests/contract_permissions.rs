//! Contract permissions: docs/adr/0086-bilateral-contract-signing.md.
use domain::identity::{Permission, Role};

const EVERY_PERMISSION: [Permission; 30] = [
    Permission::ReadDeadline,
    Permission::ManageDeadline,
    Permission::ReadDeadlineProfile,
    Permission::ManageDeadlineProfile,
    Permission::ReadDeadlineInputs,
    Permission::ReadProceduralFact,
    Permission::ManageProceduralFact,
    Permission::ReadJudicialCalendar,
    Permission::ManageJudicialCalendar,
    Permission::ReadHearingResult,
    Permission::ManageHearingResult,
    Permission::ReadHearing,
    Permission::ManageHearing,
    Permission::ReadCaseStage,
    Permission::ManageCaseStage,
    Permission::ReadCaseAdministration,
    Permission::ManageCaseAdministration,
    Permission::ReadParticipant,
    Permission::ManageParticipant,
    Permission::ReadDocument,
    Permission::CreateDocument,
    Permission::AppendDocument,
    Permission::ClassifyDocument,
    Permission::SealDocument,
    Permission::VerifyDocument,
    Permission::ExportEvidence,
    Permission::VerifyAudit,
    Permission::CreateUser,
    Permission::ReadContract,
    Permission::ManageContract,
];

#[test]
fn owner_and_litigator_manage_contracts_and_paralegal_only_reads_them() {
    for (role, read, manage) in [
        (Role::Owner, true, true),
        (Role::Litigator, true, true),
        (Role::Paralegal, true, false),
        (Role::Client, false, false),
    ] {
        assert_eq!(role.allows(Permission::ReadContract), read, "{role:?}");
        assert_eq!(role.allows(Permission::ManageContract), manage, "{role:?}");
    }
}

#[test]
fn the_client_role_holds_no_permission_at_all() {
    for permission in EVERY_PERMISSION {
        assert!(!Role::Client.allows(permission), "{permission:?}");
    }
}
