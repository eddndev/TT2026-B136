use super::{inconsistent, DeadlineError};
use crate::ApplicationError;
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineResponsibleQuery {
    limit: u32,
    after_id: Option<UserId>,
}
impl DeadlineResponsibleQuery {
    pub fn new(limit: u32, after_id: Option<UserId>) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(DeadlineError::Invalid("responsible page limit").into());
        }
        Ok(Self { limit, after_id })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn after_id(self) -> Option<UserId> {
        self.after_id
    }
}

/// Currently eligible account metadata, distinct from a historical captured identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineResponsibleCandidate {
    pub id: UserId,
    pub email: String,
    pub role: Role,
}

/// A case-authorized selection page. Selection does not reserve account eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineResponsiblePage {
    pub case_id: CaseId,
    pub responsibles: Vec<DeadlineResponsibleCandidate>,
    pub has_more: bool,
    pub next_after_id: Option<UserId>,
}
impl DeadlineResponsiblePage {
    pub fn validate(
        &self,
        case_id: CaseId,
        query: DeadlineResponsibleQuery,
    ) -> Result<(), ApplicationError> {
        if self.case_id != case_id
            || self.responsibles.len() > query.limit() as usize
            || self.has_more && self.responsibles.len() != query.limit() as usize
            || self.next_after_id
                != if self.has_more {
                    self.responsibles.last().map(|row| row.id)
                } else {
                    None
                }
        {
            return Err(inconsistent(
                "responsible page scope, size or cursor differs from its request",
            ));
        }
        let mut after = query.after_id();
        for row in &self.responsibles {
            if !matches!(row.role, Role::Owner | Role::Litigator | Role::Paralegal)
                || row.email.trim().is_empty()
                || after.is_some_and(|id| row.id.as_uuid() <= id.as_uuid())
            {
                return Err(inconsistent(
                    "responsible candidate role, identity or ordering is invalid",
                ));
            }
            after = Some(row.id);
        }
        Ok(())
    }
}
