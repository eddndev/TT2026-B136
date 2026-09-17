use super::{catalog_validation::validate_preparation, receipt::inconsistent, *};
use crate::{identity::IdentityWorkflow, ApplicationError};
use domain::{
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Permission, Role, UserId},
};
use std::sync::Arc;

pub struct DeadlineProfileService {
    pub(super) store: Arc<dyn DeadlineProfileStore>,
    identity: Arc<dyn IdentityWorkflow>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
}
impl DeadlineProfileService {
    pub fn new(
        store: Arc<dyn DeadlineProfileStore>,
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
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
    ) -> Result<PreparedDeadlineProfileChange, ApplicationError> {
        command.result_revision()?;
        match &command.change {
            DeadlineProfileChange::Publish { definition }
            | DeadlineProfileChange::Replace { definition, .. }
                if !collection.permits_mutation(definition.scope()) =>
            {
                return Err(DeadlineProfileError::ScopeChangeForbidden.into());
            }
            _ => {}
        }
        let preparation = self.store.prepare(actor, collection, &command)?;
        let (definition, algorithm) =
            validate_preparation(self.hasher.as_ref(), collection, &command, &preparation)?;
        let definition_digest =
            deadline_profile_definition_digest(self.hasher.as_ref(), &definition);
        let submission_digest = deadline_profile_submission_digest(
            self.hasher.as_ref(),
            actor,
            &command,
            algorithm,
            definition_digest,
        );
        Ok(PreparedDeadlineProfileChange {
            actor,
            collection,
            command,
            preparation,
            definition,
            definition_digest,
            algorithm,
            submission_digest,
        })
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
    ) -> Result<DeadlineProfileDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageDeadlineProfile)?;
        let prepared = self.prepared(actor.0, collection, command)?;
        self.same_actor(token, actor, Permission::ManageDeadlineProfile)?;
        Ok(DeadlineProfileDraft {
            collection,
            actor: actor.0,
            result_revision: prepared.command.result_revision()?,
            initial_scope: prepared.definition.scope().clone(),
            command: prepared.command,
            definition: prepared.definition,
            definition_digest: prepared.definition_digest,
            algorithm: prepared.algorithm,
            submission_digest: prepared.submission_digest,
        })
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
        expected: Sha256Digest,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageDeadlineProfile)?;
        let prepared = self.prepared(actor.0, collection, command.clone())?;
        if prepared.submission_digest != expected {
            return Err(DeadlineProfileError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor, Permission::ManageDeadlineProfile)?;
        let expected_definition = prepared.definition.clone();
        let expected_definition_digest = prepared.definition_digest;
        let expected_algorithm = prepared.algorithm;
        let result = self.store.commit(actor.0, prepared)?;
        deadline_profile_receipt_matches(self.hasher.as_ref(), &result)?;
        if result.id != command.profile_id
            || result.revision != command.result_revision()?
            || result.recorded_by.id != actor.0
            || result.receipt.operation_id != command.operation_id
            || result.receipt.action != command.action()
            || result.receipt.expected_revision != command.expected_revision()
            || result.receipt.submission_digest != expected
            || result.definition != expected_definition
            || result.definition_digest != expected_definition_digest
            || result.algorithm != expected_algorithm
            || result.reason.as_ref() != command.reason()
            || result.status != command.result_status()
        {
            return Err(inconsistent(
                "committed profile differs from submitted operation",
            ));
        }
        Ok(result)
    }
}
