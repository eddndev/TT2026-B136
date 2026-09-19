use super::*;
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
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

pub struct ProceduralResourceService {
    pub(super) store: Arc<dyn ProceduralResourceStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) processor: Arc<DocumentProcessor>,
    pub(super) validator: Arc<dyn DocumentFormatBatchValidator>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
    pub(super) limits: StageSupportReadLimits,
}
impl ProceduralResourceService {
    pub fn new(
        store: Arc<dyn ProceduralResourceStore>,
        identity: Arc<dyn IdentityWorkflow>,
        processor: Arc<DocumentProcessor>,
        validator: Arc<dyn DocumentFormatBatchValidator>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            processor,
            validator,
            hasher,
            clock,
            limits: StageSupportReadLimits::standard(),
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
        case_id: CaseId,
        command: ResourceCommand,
    ) -> Result<ResourceDraft, ApplicationError> {
        let actor = self.actor(token, true)?;
        command.result_revision()?;
        let result = match self
            .store
            .prepare(actor.id, case_id, &command, &self.limits)?
        {
            ResourcePreparation::Ready(material) => {
                if material.case_id != case_id {
                    return Err(inconsistent("resource preparation scope differs"));
                }
                self.prepared(&actor, command, *material)?.draft
            }
            ResourcePreparation::Replay(detail) => {
                super::validation::replay(
                    self.hasher.as_ref(),
                    actor.id,
                    case_id,
                    &command,
                    &detail,
                )?;
                super::canonical::draft_from_detail(&detail, command)
            }
        };
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: ResourceCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceDetail, ApplicationError> {
        let actor = self.actor(token, true)?;
        command.result_revision()?;
        match self
            .store
            .prepare(actor.id, case_id, &command, &self.limits)?
        {
            ResourcePreparation::Replay(detail) => {
                super::validation::replay(
                    self.hasher.as_ref(),
                    actor.id,
                    case_id,
                    &command,
                    &detail,
                )?;
                if detail.receipt.submission_digest != expected {
                    return Err(ProceduralResourceError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                Ok(*detail)
            }
            ResourcePreparation::Ready(material) => {
                if material.case_id != case_id {
                    return Err(inconsistent("resource preparation scope differs"));
                }
                let prepared = self.prepared(&actor, command.clone(), *material)?;
                if prepared.draft.submission_digest != expected {
                    return Err(ProceduralResourceError::SubmissionMismatch.into());
                }
                self.reauthenticate(token, &actor)?;
                let reviewed = prepared.draft.clone();
                let earliest = prepared.material.base.as_ref().map(|b| b.recorded_at);
                let detail = self.store.commit(actor.id, case_id, prepared)?;
                resource_receipt_matches(self.hasher.as_ref(), &detail)?;
                if detail.case_id != case_id
                    || detail.id != command.resource_id
                    || super::canonical::draft_from_detail(
                        &detail,
                        super::validation::resource_command_from_detail(&detail)?,
                    ) != reviewed
                    || earliest.is_some_and(|at| detail.recorded_at < at)
                {
                    return Err(inconsistent(
                        "committed resource differs from the reviewed command",
                    ));
                }
                Ok(detail)
            }
        }
    }
}
