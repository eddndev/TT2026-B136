use domain::identity::Permission;

/// Permission and audit vocabulary for the case-local directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantAction {
    Create,
    Replace,
    ChangeStatus,
    List,
    Read,
    History,
}

impl ParticipantAction {
    pub const fn permission(self) -> Permission {
        match self {
            Self::Create | Self::Replace | Self::ChangeStatus => Permission::ManageParticipant,
            Self::List | Self::Read | Self::History => Permission::ReadParticipant,
        }
    }

    pub const fn audit_action(self) -> &'static str {
        match self {
            Self::Create => "participant.created",
            Self::Replace => "participant.updated",
            Self::ChangeStatus => "participant.directory_status_changed",
            Self::List => "participant.listed",
            Self::Read => "participant.read",
            Self::History => "participant.history_listed",
        }
    }
}
