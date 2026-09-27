use super::*;
use crate::identity::IdentityWorkflow;
use domain::identity::Role;
use std::sync::Arc;

pub struct DashboardService {
    store: Arc<dyn DashboardStore>,
    identity: Arc<dyn IdentityWorkflow>,
}
impl DashboardService {
    pub fn new(store: Arc<dyn DashboardStore>, identity: Arc<dyn IdentityWorkflow>) -> Self {
        Self { store, identity }
    }
}
impl DashboardWorkflow for DashboardService {
    fn read(&self, token: &str) -> Result<DashboardSnapshot, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        let scope = match actor.role {
            Role::Owner => DashboardScope::Office,
            Role::Litigator => DashboardScope::AssignedCases,
            _ => return Err(ApplicationError::PermissionDenied),
        };
        let result = self.store.read(actor.id)?;
        result.validate(scope)?;
        if self.identity.authenticate(token)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(result)
    }
}
