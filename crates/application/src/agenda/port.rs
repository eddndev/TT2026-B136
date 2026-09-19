use super::{AgendaPage, AgendaQuery};
use crate::ApplicationError;
use domain::identity::UserId;

pub trait AgendaStore: Send + Sync {
    /// Authorize both the actor and each candidate, verify current captures,
    /// and audit the read in one transaction. Sample the page timestamp after
    /// acquiring the consistency lock and reuse it for every deadline.
    /// Inspect at most MAX_AGENDA_CANDIDATES; an incomplete page may be empty
    /// when examined candidates no longer have an operational due date.
    fn list(&self, actor: UserId, query: AgendaQuery) -> Result<AgendaPage, ApplicationError>;
}

pub trait AgendaWorkflow: Send + Sync {
    fn list(&self, token: &str, query: AgendaQuery) -> Result<AgendaPage, ApplicationError>;
}
