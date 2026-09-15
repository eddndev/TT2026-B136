use domain::identity::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStageAction {
    Read,
    History,
    Adopt,
    Transition,
}

impl CaseStageAction {
    pub const fn permission(self) -> Permission {
        match self {
            Self::Read | Self::History => Permission::ReadCaseStage,
            Self::Adopt | Self::Transition => Permission::ManageCaseStage,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "case.stage_read",
            Self::History => "case.stage_history_read",
            Self::Adopt => "case.stage_adopted",
            Self::Transition => "case.stage_transitioned",
        }
    }
}
