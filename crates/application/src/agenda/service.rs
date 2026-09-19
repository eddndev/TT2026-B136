use super::{AgendaKind, AgendaPage, AgendaQuery, AgendaStore, AgendaWorkflow};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::identity::Permission;
use std::sync::Arc;

pub struct AgendaService {
    store: Arc<dyn AgendaStore>,
    identity: Arc<dyn IdentityWorkflow>,
}
impl AgendaService {
    pub fn new(store: Arc<dyn AgendaStore>, identity: Arc<dyn IdentityWorkflow>) -> Self {
        Self { store, identity }
    }
    fn actor(&self, token: &str, kind: AgendaKind) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        let allowed = match kind {
            AgendaKind::All => {
                actor.role.allows(Permission::ReadHearing)
                    && actor.role.allows(Permission::ReadDeadline)
            }
            AgendaKind::Hearing => actor.role.allows(Permission::ReadHearing),
            AgendaKind::Deadline => actor.role.allows(Permission::ReadDeadline),
        };
        if !allowed {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
}
impl AgendaWorkflow for AgendaService {
    fn list(&self, token: &str, query: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
        let actor = self.actor(token, query.kind())?;
        let page = self.store.list(actor.id, query)?;
        page.validate(&query)?;
        if self.actor(token, query.kind())? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(page)
    }
}
