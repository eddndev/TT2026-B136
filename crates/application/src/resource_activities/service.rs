use super::*;
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
};
use std::sync::Arc;

pub struct ResourceActivityService {
    pub(super) store: Arc<dyn ResourceActivityStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
}
impl ResourceActivityService {
    pub fn new(
        store: Arc<dyn ResourceActivityStore>,
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
    pub(super) fn actor(&self, token: &str, write: bool) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if actor.role == Role::Client || (write && actor.role == Role::Paralegal) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    pub(super) fn reauthenticate(
        &self,
        token: &str,
        actor: &Principal,
    ) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
    ) -> Result<ResourceActivityDraft, ApplicationError> {
        let actor = self.actor(token, true)?;
        command.result_revision()?;
        let draft = match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceActivityPreparation::Ready(material) => {
                super::preparation::prepare(
                    self.hasher.clone(),
                    &actor,
                    case,
                    resource,
                    command,
                    *material,
                )?
                .draft
            }
            ResourceActivityPreparation::Replay(detail) => {
                super::validation::replay(
                    self.hasher.as_ref(),
                    &actor,
                    case,
                    resource,
                    &command,
                    &detail,
                )?;
                super::canonical::draft_from_detail(&detail)?
            }
        };
        self.reauthenticate(token, &actor)?;
        Ok(draft)
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        let actor = self.actor(token, true)?;
        command.result_revision()?;
        match self.store.prepare(actor.id, case, resource, &command)? {
            ResourceActivityPreparation::Replay(detail) => {
                super::validation::replay(
                    self.hasher.as_ref(),
                    &actor,
                    case,
                    resource,
                    &command,
                    &detail,
                )?;
                if detail.receipt.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                Ok(*detail)
            }
            ResourceActivityPreparation::Ready(material) => {
                let prepared = super::preparation::prepare(
                    self.hasher.clone(),
                    &actor,
                    case,
                    resource,
                    command,
                    *material,
                )?;
                if prepared.draft.submission_digest != expected {
                    return Err(ResourceActivityError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                let reviewed = prepared.draft.clone();
                let earliest = prepared
                    .material
                    .base
                    .as_ref()
                    .map(|v| v.recorded_at)
                    .unwrap_or(prepared.material.resource_head.recorded_at)
                    .max(prepared.material.resource_head.recorded_at);
                let detail = self.store.commit(actor.id, case, resource, prepared)?;
                resource_activity_receipt_matches(self.hasher.as_ref(), &detail)?;
                if super::canonical::draft_from_detail(&detail)? != reviewed
                    || detail.recorded_at < earliest
                {
                    return Err(inconsistent(
                        "committed association differs from reviewed capture",
                    ));
                }
                Ok(detail)
            }
        }
    }
}
