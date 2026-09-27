use super::inconsistent;
use super::*;
use crate::{
    identity::{IdentityWorkflow, Principal},
    resource_activities::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
};
use std::sync::Arc;
pub struct ResourceDeadlineService {
    store: Arc<dyn ResourceDeadlineStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl ResourceDeadlineService {
    pub fn new(
        store: Arc<dyn ResourceDeadlineStore>,
        identity: Arc<dyn IdentityWorkflow>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            hasher,
            clock,
        }
    }
    fn actor(&self, token: &str) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !matches!(actor.role, Role::Owner | Role::Litigator) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.actor(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn replay(
        &self,
        actor: &Principal,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceDeadlineCommand,
        result: &ResourceDeadlineResult,
    ) -> Result<ResourceDeadlineDraft, ApplicationError> {
        let draft = resource_deadline_result_draft(
            self.hasher.as_ref(),
            actor,
            case,
            resource,
            command,
            result,
        )?;
        if result.association.recorded_at > self.clock.now() {
            return Err(inconsistent("contextual receipt is from the future"));
        }
        Ok(draft)
    }
}
impl ResourceDeadlineWorkflow for ResourceDeadlineService {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlineDraft, ApplicationError> {
        let actor = self.actor(token)?;
        super::preparation::register(&command)?;
        let draft = match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceDeadlinePreparation::Ready(material) => {
                prepare_resource_deadline_change(
                    self.hasher.clone(),
                    &actor,
                    case,
                    resource,
                    command,
                    *material,
                )?
                .draft
            }
            ResourceDeadlinePreparation::Replay(result) => {
                self.replay(&actor, case, resource, &command, &result)?
            }
        };
        self.reauthenticate(token, &actor)?;
        Ok(draft)
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        let actor = self.actor(token)?;
        super::preparation::register(&command)?;
        match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceDeadlinePreparation::Replay(result) => {
                let draft = self.replay(&actor, case, resource, &command, &result)?;
                if draft.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                Ok(*result)
            }
            ResourceDeadlinePreparation::Ready(material) => {
                let prepared = prepare_resource_deadline_change(
                    self.hasher.clone(),
                    &actor,
                    case,
                    resource,
                    command.clone(),
                    *material,
                )?;
                if prepared.draft.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                let draft = prepared.draft.clone();
                self.reauthenticate(token, &actor)?;
                let result = self.store.commit(actor.id, case, resource, prepared)?;
                if self.replay(&actor, case, resource, &command, &result)? != draft {
                    return Err(inconsistent(
                        "committed contextual receipt differs from review",
                    ));
                }
                self.reauthenticate(token, &actor)?;
                Ok(result)
            }
        }
    }
}
