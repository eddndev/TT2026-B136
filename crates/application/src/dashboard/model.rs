use crate::ApplicationError;
use domain::{clock::OffsetDateTime, identity::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardScope {
    Office,
    AssignedCases,
}
impl DashboardScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Office => "office",
            Self::AssignedCases => "assigned_cases",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardWorkload {
    pub user_id: UserId,
    pub email: String,
    pub active_cases: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardSnapshot {
    pub scope: DashboardScope,
    pub checked_at: OffsetDateTime,
    pub active_cases: u64,
    pub pending_contracts: u64,
    pub deadlines_overdue: u64,
    pub deadlines_due_48h: u64,
    pub deadlines_due_7d: u64,
    pub deadlines_unresolved: u64,
    pub workload: Vec<DashboardWorkload>,
}
impl DashboardSnapshot {
    pub fn validate(&self, expected_scope: DashboardScope) -> Result<(), ApplicationError> {
        if self.scope != expected_scope
            || !(1..=9999).contains(&self.checked_at.year())
            || self.deadlines_due_48h > self.deadlines_due_7d
            || self
                .workload
                .windows(2)
                .any(|p| p[0].user_id.as_uuid() >= p[1].user_id.as_uuid())
            || self.workload.iter().any(|v| {
                v.active_cases > self.active_cases
                    || v.email.is_empty()
                    || v.email.chars().any(char::is_control)
                    || (self.scope == DashboardScope::AssignedCases && v.active_cases == 0)
            })
        {
            return Err(ApplicationError::Port(
                "dashboard snapshot is inconsistent".into(),
            ));
        }
        Ok(())
    }
}
