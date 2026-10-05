use super::{record_workflow_evidence, workflow_evidence as evidence, *};
use crate::measure_corrections::MeasureReadInventory;
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::precautionary_hearings::{
    PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::Role,
};
use std::sync::Arc;

pub struct PrecautionaryHearingRecordReadService {
    store: Arc<dyn PrecautionaryHearingRecordReadStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PrecautionaryHearingRecordReadService {
    pub fn new(
        store: Arc<dyn PrecautionaryHearingRecordReadStore>,
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
        if actor.role == Role::Client {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }
    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn observation(
        &self,
        lower: Option<OffsetDateTime>,
    ) -> Result<OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        evidence::clock(at)?;
        if lower.is_some_and(|lower| at < lower) {
            return Err(evidence::invalid("read clock regressed"));
        }
        Ok(at)
    }
    fn operation(
        &self,
        result: &PrecautionaryHearingRecordStoredOperation,
        case_id: CaseId,
    ) -> Result<(), ApplicationError> {
        if result.capture.review.case_id != case_id {
            return Err(evidence::invalid("read case differs"));
        }
        record_workflow_evidence::operation(self.hasher.as_ref(), result)
    }
    fn disclose(
        &self,
        token: &str,
        actor: &Principal,
        floor: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        self.reauthenticate(token, actor)?;
        self.observation(Some(floor))?;
        Ok(())
    }
}

impl PrecautionaryHearingRecordReadWorkflow for PrecautionaryHearingRecordReadService {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: PrecautionaryHearingReadQuery,
    ) -> Result<PrecautionaryHearingRecordPage, ApplicationError> {
        let actor = self.actor(token)?;
        let mut floor = self.observation(None)?;
        let page = self.store.list(&actor, case_id, query)?;
        let last = page
            .items
            .last()
            .map(|r| r.capture.review.command.hearing_id);
        if page.case_id != case_id
            || page.items.len() > usize::from(query.limit())
            || (page.has_more
                && (page.items.len() != usize::from(query.limit())
                    || page.next_after_id.is_none()
                    || page.next_after_id != last))
            || (!page.has_more && page.next_after_id.is_some())
        {
            return Err(evidence::invalid(
                "read page scope, length or continuation differs",
            ));
        }
        let mut prior = query.after_id();
        let mut inventory = MeasureReadInventory::default();
        for row in &page.items {
            let id = row.capture.review.command.hearing_id;
            if prior.is_some_and(|before| before.as_uuid() >= id.as_uuid()) {
                return Err(evidence::invalid("read page order differs"));
            }
            self.operation(row, case_id)?;
            inventory
                .add_hearing_operation(row)
                .map_err(|error| evidence::invalid(&error.to_string()))?;
            floor = floor.max(row.capture.recorded_at);
            prior = Some(id);
        }
        self.disclose(token, &actor, floor)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        hearing: PrecautionaryHearingId,
        revision: Option<PrecautionaryHearingRevision>,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let result = self.store.get(&actor, case_id, hearing, revision)?;
        self.operation(&result, case_id)?;
        if result.capture.review.command.hearing_id != hearing
            || revision.is_some_and(|r| r != result.capture.review.result_revision)
        {
            return Err(evidence::invalid("exact read identity or revision differs"));
        }
        self.disclose(token, &actor, started.max(result.capture.recorded_at))?;
        Ok(result)
    }
    fn get_operation(
        &self,
        token: &str,
        case_id: CaseId,
        operation: PrecautionaryHearingOperationId,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let result = self.store.get_operation(&actor, case_id, operation)?;
        self.operation(&result, case_id)?;
        if result.capture.review.command.operation_id != operation {
            return Err(evidence::invalid("exact read operation differs"));
        }
        self.disclose(token, &actor, started.max(result.capture.recorded_at))?;
        Ok(result)
    }
}
