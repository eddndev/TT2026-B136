use super::{cursor, CaseId, MemberError, Role, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatusFilter {
    Active,
    Inactive,
    All,
}

impl UserStatusFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberSelection {
    Assigned,
    Available,
}

impl MemberSelection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Available => "available",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserQuery {
    limit: u32,
    status: UserStatusFilter,
    role: Option<Role>,
    email_prefix: Option<String>,
    after_id: Option<UserId>,
}

impl UserQuery {
    pub fn new(
        limit: u32,
        status: UserStatusFilter,
        role: Option<Role>,
        email_prefix: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<Self, MemberError> {
        cursor::limit(limit)?;
        let mut query = Self {
            limit,
            status,
            role,
            email_prefix: cursor::prefix(email_prefix)?,
            after_id: None,
        };
        query.after_id = cursor::after(cursor, 1, |id| query.cursor_after(id))?;
        Ok(query)
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn status(&self) -> UserStatusFilter {
        self.status
    }
    pub const fn role(&self) -> Option<Role> {
        self.role
    }
    pub fn email_prefix(&self) -> Option<&str> {
        self.email_prefix.as_deref()
    }
    pub const fn after_id(&self) -> Option<UserId> {
        self.after_id
    }
    pub fn cursor_after(&self, id: UserId) -> String {
        format!(
            "ud1:{id}:{}:{}:{}",
            self.status.as_str(),
            cursor::role(self.role),
            cursor::encoded_prefix(self.email_prefix())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseMemberQuery {
    case_id: CaseId,
    limit: u32,
    selection: MemberSelection,
    role: Option<Role>,
    email_prefix: Option<String>,
    after_id: Option<UserId>,
}

impl CaseMemberQuery {
    pub fn new(
        case_id: CaseId,
        limit: u32,
        selection: MemberSelection,
        role: Option<Role>,
        email_prefix: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<Self, MemberError> {
        cursor::limit(limit)?;
        let mut query = Self {
            case_id,
            limit,
            selection,
            role,
            email_prefix: cursor::prefix(email_prefix)?,
            after_id: None,
        };
        query.after_id = cursor::after(cursor, 2, |id| query.cursor_after(id))?;
        Ok(query)
    }
    pub const fn case_id(&self) -> CaseId {
        self.case_id
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn selection(&self) -> MemberSelection {
        self.selection
    }
    pub const fn role(&self) -> Option<Role> {
        self.role
    }
    pub fn email_prefix(&self) -> Option<&str> {
        self.email_prefix.as_deref()
    }
    pub const fn after_id(&self) -> Option<UserId> {
        self.after_id
    }
    pub fn cursor_after(&self, id: UserId) -> String {
        format!(
            "cm1:{}:{id}:{}:{}:{}",
            self.case_id,
            self.selection.as_str(),
            cursor::role(self.role),
            cursor::encoded_prefix(self.email_prefix())
        )
    }
}
