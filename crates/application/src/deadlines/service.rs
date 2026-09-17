use super::*;
use crate::{identity::IdentityWorkflow, ApplicationError};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Permission, Role, UserId},
};
use std::sync::Arc;

pub struct DeadlineService {
    pub(super) store: Arc<dyn DeadlineStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
}
impl DeadlineService {
    pub fn new(
        store: Arc<dyn DeadlineStore>,
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
    pub(super) fn actor(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<(UserId, Role), ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok((actor.id, actor.role))
    }
    pub(super) fn same_actor(
        &self,
        token: &str,
        actor: (UserId, Role),
        permission: Permission,
    ) -> Result<(), ApplicationError> {
        if self.actor(token, permission)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn prepared(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: DeadlineCommand,
    ) -> Result<PreparedDeadlineChange, ApplicationError> {
        command.result_revision()?;
        let preparation = self.store.prepare(actor, case_id, &command)?;
        prepare_deadline_change(self.hasher.as_ref(), actor, case_id, command, preparation)
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
    ) -> Result<DeadlineDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageDeadline)?;
        let prepared = self.prepared(actor.0, case_id, command)?;
        self.same_actor(token, actor, Permission::ManageDeadline)?;
        draft(&prepared)
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
        expected: Sha256Digest,
    ) -> Result<DeadlineDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageDeadline)?;
        let prepared = self.prepared(actor.0, case_id, command)?;
        if prepared.submission_digest() != expected {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor, Permission::ManageDeadline)?;
        let reviewed = draft(&prepared)?;
        let committed = self.store.commit(actor.0, prepared)?;
        super::service_validation::validate_commit(self.hasher.as_ref(), &reviewed, &committed)?;
        Ok(committed)
    }
}
fn draft(prepared: &PreparedDeadlineChange) -> Result<DeadlineDraft, ApplicationError> {
    Ok(DeadlineDraft {
        case_id: prepared.case_id(),
        actor: prepared.actor(),
        command: prepared.command().clone(),
        result_revision: prepared.command().result_revision()?,
        definition: prepared.definition().clone(),
        calculation: prepared.calculation().clone(),
        responsible: prepared.responsible().clone(),
        attention: prepared.attention().clone(),
        status: prepared.status(),
        review_digest: prepared.review_digest(),
        capture_digest: prepared.capture_digest(),
        submission_digest: prepared.submission_digest(),
    })
}
