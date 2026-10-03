use super::*;
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    identity::{Role, UserId},
    procedural_resources::ResourceId,
};
use std::sync::Arc;

pub trait ResourceHearingStore: Send + Sync {
    /// Authorize current case membership before reading any material. Resolve
    /// exact historical resource/act captures and the current head independently.
    /// Bound participants to 32 and require their selected revisions to be current.
    /// The support must already be admitted in the selected resource or act.
    /// This operation performs no write and does not claim durable creation.
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceHearingCommand,
    ) -> Result<ResourceHearingMaterial, ApplicationError>;
}

pub struct ResourceHearingService {
    store: Arc<dyn ResourceHearingStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl ResourceHearingService {
    pub fn new(
        store: Arc<dyn ResourceHearingStore>,
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
        if !matches!(actor.role, Role::Owner | Role::Litigator) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    pub fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
    ) -> Result<ResourceHearingDraft, ApplicationError> {
        let actor = self.actor(token)?;
        let material = self.store.prepare(actor.id, case, resource, &command)?;
        let draft = super::preparation::prepare(
            self.hasher.as_ref(),
            &actor,
            case,
            resource,
            command,
            material,
        )?;
        if self.actor(token)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(draft)
    }
}
