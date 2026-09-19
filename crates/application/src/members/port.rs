use super::{
    CaseId, CaseMemberPage, CaseMemberQuery, OffsetDateTime, UserAccessChange, UserId, UserPage,
    UserQuery, UserSummary,
};
use crate::ApplicationError;

pub trait MemberWorkflow: Send + Sync {
    fn list(&self, token: &str, query: UserQuery) -> Result<UserPage, ApplicationError>;
    fn get(&self, token: &str, id: UserId) -> Result<UserSummary, ApplicationError>;
    fn change_access(
        &self,
        token: &str,
        id: UserId,
        change: UserAccessChange,
    ) -> Result<UserSummary, ApplicationError>;
    fn list_case_members(
        &self,
        token: &str,
        case_id: CaseId,
        query: CaseMemberQuery,
    ) -> Result<CaseMemberPage, ApplicationError>;
}

/// Revalidates the active Owner inside the common audited transaction before lookup.
/// Reads commit audit before returning rows. Access changes commit state, revision,
/// authentication generation and audit together, preserving credential material.
/// The request time is a lower bound; committed time is captured after locking.
pub trait MemberStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        query: UserQuery,
        at: OffsetDateTime,
    ) -> Result<UserPage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        id: UserId,
        at: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError>;
    fn change_access(
        &self,
        actor: UserId,
        id: UserId,
        change: UserAccessChange,
        at: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError>;
    fn list_case_members(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: CaseMemberQuery,
        at: OffsetDateTime,
    ) -> Result<CaseMemberPage, ApplicationError>;
}
