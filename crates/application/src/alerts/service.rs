use super::*;
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::identity::Permission;
use std::sync::Arc;

pub struct AlertService {
    store: Arc<dyn AlertStore>,
    identity: Arc<dyn IdentityWorkflow>,
}
impl AlertService {
    pub fn new(store: Arc<dyn AlertStore>, identity: Arc<dyn IdentityWorkflow>) -> Self {
        Self { store, identity }
    }
    fn actor(&self, token: &str) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(Permission::ReadDeadline)
            || !actor.role.allows(Permission::ReadHearing)
        {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
}
impl AlertWorkflow for AlertService {
    fn preferences(&self, token: &str) -> Result<AlertPreferences, ApplicationError> {
        let actor = self.actor(token)?;
        let preferences = self.store.preferences(actor.id)?;
        preferences.validate(actor.id)?;
        self.reauthenticate(token, &actor)?;
        Ok(preferences)
    }
    fn save_preferences(
        &self,
        token: &str,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError> {
        let actor = self.actor(token)?;
        if command.expected_revision == u32::MAX {
            return Err(AlertError::Invalid("preference revision").into());
        }
        let preferences = self.store.save_preferences(actor.id, command.clone())?;
        preferences.validate_command(actor.id, &command)?;
        self.reauthenticate(token, &actor)?;
        Ok(preferences)
    }
    fn list(&self, token: &str, query: AlertQuery) -> Result<AlertPage, ApplicationError> {
        let actor = self.actor(token)?;
        let page = self.store.list(actor.id, query)?;
        page.validate(actor.id, &query)?;
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn get(&self, token: &str, id: AlertId) -> Result<AlertDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let detail = self.store.get(actor.id, id)?;
        detail.alert.validate(actor.id, detail.checked_at)?;
        if detail.alert.id != id {
            return Err(validation::stored("notification detail identity differs"));
        }
        self.reauthenticate(token, &actor)?;
        Ok(detail)
    }
    fn mark_read(
        &self,
        token: &str,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError> {
        let actor = self.actor(token)?;
        let receipt = self.store.mark_read(actor.id, command)?;
        receipt.alert.validate(actor.id, receipt.checked_at)?;
        if receipt.operation_id != command.operation_id
            || receipt.alert.id != command.alert_id
            || receipt.alert.read_at.is_none()
        {
            return Err(validation::stored("notification read receipt differs"));
        }
        self.reauthenticate(token, &actor)?;
        Ok(receipt)
    }
}
