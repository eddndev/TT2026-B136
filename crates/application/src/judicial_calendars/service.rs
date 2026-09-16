use super::{receipt::inconsistent, validation::validate_preparation, *};
use crate::{identity::IdentityWorkflow, ApplicationError};
use domain::{
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Permission, UserId},
};
use std::sync::Arc;
pub struct JudicialCalendarService {
    pub(super) store: Arc<dyn JudicialCalendarStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
}
impl JudicialCalendarService {
    pub fn new(
        store: Arc<dyn JudicialCalendarStore>,
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
    ) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }
    fn same_actor(&self, token: &str, actor: UserId) -> Result<(), ApplicationError> {
        if self.actor(token, Permission::ManageJudicialCalendar)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn prepared(
        &self,
        actor: UserId,
        command: JudicialCalendarCommand,
    ) -> Result<PreparedJudicialCalendarChange, ApplicationError> {
        command.result_revision()?;
        let preparation = self.store.prepare(actor, &command)?;
        let values = validate_preparation(self.hasher.as_ref(), &command, &preparation)?;
        let values_digest = judicial_calendar_values_digest(self.hasher.as_ref(), &values);
        let submission_digest = judicial_calendar_submission_digest(
            self.hasher.as_ref(),
            actor,
            &command,
            values_digest,
        );
        Ok(PreparedJudicialCalendarChange {
            actor,
            command,
            preparation,
            values,
            values_digest,
            submission_digest,
        })
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
    ) -> Result<JudicialCalendarDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageJudicialCalendar)?;
        let prepared = self.prepared(actor, command)?;
        self.same_actor(token, actor)?;
        Ok(JudicialCalendarDraft {
            actor,
            result_revision: prepared.command.result_revision()?,
            initial_scope: prepared.values.scope().clone(),
            command: prepared.command,
            values: prepared.values,
            values_digest: prepared.values_digest,
            submission_digest: prepared.submission_digest,
        })
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
        expected: Sha256Digest,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageJudicialCalendar)?;
        let prepared = self.prepared(actor, command.clone())?;
        if prepared.submission_digest != expected {
            return Err(JudicialCalendarError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor)?;
        let expected_values = prepared.values.clone();
        let expected_values_digest = prepared.values_digest;
        let result = self.store.commit(actor, prepared)?;
        judicial_calendar_receipt_matches(self.hasher.as_ref(), &result)?;
        if result.id != command.calendar_id
            || result.revision != command.result_revision()?
            || result.recorded_by.id != actor
            || result.receipt.operation_id != command.operation_id
            || result.receipt.action != command.action()
            || result.receipt.expected_revision != command.expected_revision()
            || result.receipt.submission_digest != expected
            || result.values != expected_values
            || result.values_digest != expected_values_digest
            || result.reason.as_ref() != command.reason()
            || result.status != command.result_status()
        {
            return Err(inconsistent(
                "committed calendar differs from submitted operation",
            ));
        }
        Ok(result)
    }
}
