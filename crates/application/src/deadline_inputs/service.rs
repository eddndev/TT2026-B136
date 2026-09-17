use super::*;
use crate::identity::{IdentityWorkflow, Principal};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::TriggeredArithmetic,
    identity::{Permission, UserId},
};
use std::sync::Arc;

/// A checked read, not a persisted evaluation or permission for a later mutation.
#[derive(Debug)]
pub struct PreparedDeadlineInputs {
    actor: UserId,
    request: DeadlineInputRequest,
    material: DeadlineInputMaterial,
    calculation: TriggeredArithmetic,
}
impl PreparedDeadlineInputs {
    pub fn actor(&self) -> UserId {
        self.actor
    }
    pub fn request(&self) -> &DeadlineInputRequest {
        &self.request
    }
    pub fn material(&self) -> &DeadlineInputMaterial {
        &self.material
    }
    pub fn calculation(&self) -> &TriggeredArithmetic {
        &self.calculation
    }
}

pub struct DeadlineInputService {
    store: Arc<dyn DeadlineInputStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl DeadlineInputService {
    pub fn new(
        store: Arc<dyn DeadlineInputStore>,
        identity: Arc<dyn IdentityWorkflow>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            hasher,
        }
    }
    fn actor(&self, token: &str) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(Permission::ReadDeadlineInputs) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    pub fn prepare(
        &self,
        token: &str,
        request: DeadlineInputRequest,
    ) -> Result<PreparedDeadlineInputs, ApplicationError> {
        let actor = self.actor(token)?;
        let material = self.store.load(actor.id, &request)?;
        let calculation = check_deadline_inputs(self.hasher.as_ref(), &request, &material)?;
        let current = self.actor(token)?;
        if current.id != actor.id || current.role != actor.role {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(PreparedDeadlineInputs {
            actor: actor.id,
            request,
            material,
            calculation,
        })
    }
}
