use super::*;
use crate::{
    identity::{IdentityWorkflow, Principal},
    resource_activities::ResourceActivityError,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
    procedural_resources::ResourceId,
};
use std::sync::Arc;

pub struct ResourceHearingService {
    store: Arc<dyn ResourceHearingStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl ResourceHearingWorkflow for ResourceHearingService {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
    ) -> Result<ResourceHearingDraft, ApplicationError> {
        ResourceHearingService::prepare(self, token, case, resource, command)
    }

    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        ResourceHearingService::submit(
            self,
            token,
            case,
            resource,
            command,
            expected_submission_digest,
        )
    }
}
impl ResourceHearingService {
    pub fn new(
        store: Arc<dyn ResourceHearingStore>,
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
        command: &ResourceHearingCommand,
        result: &ResourceHearingCreation,
    ) -> Result<ResourceHearingDraft, ApplicationError> {
        resource_hearing_creation_matches(self.hasher.as_ref(), result)?;
        let draft = &result.hearing.review;
        if draft.case_id != case
            || draft.command.resource.id != resource
            || draft.recorded_by.id != actor.id
            || draft.command != *command
        {
            return Err(ResourceActivityError::OperationConflict.into());
        }
        if result.hearing.recorded_at > self.clock.now() {
            return Err(super::receipt::inconsistent(
                "resource hearing receipt is from the future",
            ));
        }
        Ok(draft.clone())
    }
    pub fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
    ) -> Result<ResourceHearingDraft, ApplicationError> {
        let actor = self.actor(token)?;
        let draft = match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceHearingPreparation::Ready(material) => prepare_resource_hearing_change(
                self.hasher.clone(),
                &actor,
                case,
                resource,
                command,
                *material,
            )?
            .draft()
            .clone(),
            ResourceHearingPreparation::Replay(result) => {
                self.replay(&actor, case, resource, &command, &result)?
            }
        };
        self.reauthenticate(token, &actor)?;
        Ok(draft)
    }
    pub fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceHearingCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        let actor = self.actor(token)?;
        match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceHearingPreparation::Replay(result) => {
                let draft = self.replay(&actor, case, resource, &command, &result)?;
                if draft.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                Ok(*result)
            }
            ResourceHearingPreparation::Ready(material) => {
                let prepared = prepare_resource_hearing_change(
                    self.hasher.clone(),
                    &actor,
                    case,
                    resource,
                    command.clone(),
                    *material,
                )?;
                let draft = prepared.draft().clone();
                if draft.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                let result = self.store.commit(actor.id, case, resource, prepared)?;
                if self.replay(&actor, case, resource, &command, &result)? != draft {
                    return Err(super::receipt::inconsistent(
                        "committed hearing differs from reviewed preparation",
                    ));
                }
                self.reauthenticate(token, &actor)?;
                Ok(result)
            }
        }
    }
}
