use super::{preparation::*, receipt::inconsistent, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::IdentityWorkflow,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Permission, UserId},
};
use std::sync::Arc;

pub struct ProceduralFactService {
    pub(super) store: Arc<dyn ProceduralFactStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}
impl ProceduralFactService {
    pub fn new(
        store: Arc<dyn ProceduralFactStore>,
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
        if self.actor(token, Permission::ManageProceduralFact)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn prepared(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: ProceduralFactCommand,
    ) -> Result<PreparedFactChange, ApplicationError> {
        command.result_revision()?;
        let mut preparation = self.store.prepare(actor, case_id, &command, &self.limits)?;
        let values =
            validate_preparation(self.hasher.as_ref(), case_id, &command, &mut preparation)?;
        let sources = if command.action() == FactAction::Withdraw {
            preparation
                .base
                .as_ref()
                .ok_or_else(|| inconsistent("withdrawal lacks its validated sources"))?
                .sources
                .clone()
        } else {
            let mut sources = resolve_sources(self.hasher.as_ref(), &preparation, &values)?;
            let formats = if preparation.records.is_empty() {
                vec![]
            } else {
                self.processor.validate_support_batch(
                    &preparation.records,
                    &self.limits,
                    self.validator.as_ref(),
                )?
            };
            sources.direct_supports = admitted_supports(&preparation, formats)?;
            if let Some(base) = &preparation.base {
                validate_retained(&base.sources, &sources)?;
            }
            sources
        };
        super::receipt::validate_source_selection(
            case_id,
            &FactSourceSelection::from_values(&values),
            &sources,
        )?;
        let values_digest = fact_values_digest(self.hasher.as_ref(), &values);
        let sources_digest = fact_sources_digest(self.hasher.as_ref(), &sources)?;
        let submission_digest = fact_submission_digest(
            self.hasher.as_ref(),
            actor,
            case_id,
            &command,
            values_digest,
            sources_digest,
        )?;
        Ok(PreparedFactChange {
            command,
            preparation,
            values,
            values_digest,
            sources,
            sources_digest,
            submission_digest,
        })
    }
    pub(super) fn prepare_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
    ) -> Result<FactDraft, ApplicationError> {
        let actor = self.actor(token, Permission::ManageProceduralFact)?;
        let prepared = self.prepared(actor, case_id, command)?;
        self.same_actor(token, actor)?;
        Ok(FactDraft {
            case_id,
            actor,
            result_revision: prepared.command.result_revision()?,
            command: prepared.command,
            values: prepared.values,
            values_digest: prepared.values_digest,
            sources: prepared.sources,
            sources_digest: prepared.sources_digest,
            submission_digest: prepared.submission_digest,
            observed_administration: prepared.preparation.observed_administration,
        })
    }
    pub(super) fn submit_command(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
        expected: Sha256Digest,
    ) -> Result<FactDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageProceduralFact)?;
        let prepared = self.prepared(actor, case_id, command.clone())?;
        if prepared.submission_digest != expected {
            return Err(ProceduralFactError::SubmissionMismatch.into());
        }
        self.same_actor(token, actor)?;
        let administration = prepared.preparation.observed_administration.clone();
        let result = self.store.commit(actor, case_id, prepared)?;
        fact_receipt_matches(self.hasher.as_ref(), &result)?;
        let snapshot = &result.snapshot;
        let metadata = snapshot.metadata();
        if snapshot.case_id() != case_id
            || snapshot.target() != command.target()
            || metadata.recorded_by.id != actor
            || metadata.revision != command.result_revision()?
            || metadata.receipt.operation_id != command.operation_id()
            || metadata.receipt.action != command.action()
            || metadata.receipt.submission_digest != expected
        {
            return Err(inconsistent(
                "committed receipt differs from submitted command",
            ));
        }
        validate_fact_administration(
            self.hasher.as_ref(),
            case_id,
            &metadata.recorded_administration,
            Some(&administration),
        )?;
        Ok(result)
    }
}
pub(super) fn support_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::StageSupportTooLarge => ProceduralFactError::SupportTooLarge.into(),
        ApplicationError::StageSupportFormatRejected => {
            ProceduralFactError::SupportFormatRejected.into()
        }
        ApplicationError::StageSupportValidationLimit => {
            ProceduralFactError::SupportValidationLimit.into()
        }
        ApplicationError::StageSupportDigestMismatch => {
            ProceduralFactError::SupportDigestMismatch.into()
        }
        ApplicationError::StageSupportChanged => ProceduralFactError::SupportChanged.into(),
        other => other,
    }
}
