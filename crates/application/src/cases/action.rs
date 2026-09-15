use domain::identity::Permission;

/// Staff administration actions with distinct audit names and role permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseAdministrationAction {
    RegisterPenal,
    Replace,
    ChangeStatus,
    List,
    Read,
    History,
}
impl CaseAdministrationAction {
    pub const fn permission(self) -> Permission {
        match self {
            Self::RegisterPenal | Self::Replace | Self::ChangeStatus => {
                Permission::ManageCaseAdministration
            }
            Self::List | Self::Read | Self::History => Permission::ReadCaseAdministration,
        }
    }
    pub const fn audit_action(self) -> &'static str {
        match self {
            Self::RegisterPenal => "case.penal_registered",
            Self::Replace => "case.administration_replaced",
            Self::ChangeStatus => "case.administrative_status_changed",
            Self::List => "case.administration_listed",
            Self::Read => "case.administration_read",
            Self::History => "case.administration_history_read",
        }
    }
}
