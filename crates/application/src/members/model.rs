use super::{CaseId, OffsetDateTime, Role, UserId};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub id: UserId,
    pub email: String,
    pub role: Role,
    pub active: bool,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPage {
    pub items: Vec<UserSummary>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseMemberItem {
    pub user: UserSummary,
    pub assigned_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseMemberPage {
    pub case_id: CaseId,
    pub items: Vec<CaseMemberItem>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserAccessChange {
    expected_revision: u64,
    role: Role,
    active: bool,
}

impl UserAccessChange {
    pub fn new(expected_revision: u64, role: Role, active: bool) -> Result<Self, MemberError> {
        if expected_revision > i64::MAX as u64 {
            return Err(MemberError::Invalid(
                "user revision exceeds its storage range",
            ));
        }
        Ok(Self {
            expected_revision,
            role,
            active,
        })
    }
    pub const fn expected_revision(self) -> u64 {
        self.expected_revision
    }
    pub const fn role(self) -> Role {
        self.role
    }
    pub const fn active(self) -> bool {
        self.active
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MemberError {
    #[error("invalid member input: {0}")]
    Invalid(&'static str),
    #[error("invalid stored member response: {0}")]
    Stored(&'static str),
    #[error("user revision changed")]
    RevisionConflict,
    #[error("at least one active owner must remain")]
    LastActiveOwner,
    #[error("user access version is exhausted")]
    AccessVersionExhausted,
}
